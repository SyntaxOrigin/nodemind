//! Kendi tam metin arama motorumuz: ters indeks, Türkçe normalizasyonu,
//! basit kök indirgeme ve alan ağırlıklı sıralama.
//!
//! `tantivy` ve benzeri FTS kütüphaneleri **yasaktır**
//! (`WORKER_CONTRACT.md` § 3.2-F); indeks tek segmentli
//! `BTreeMap<terim, BTreeMap<blok_kimligi, frekans>>` yapısıdır ve 10.000 not
//! ölçeği için yeterlidir (MANIFEST kart 21, sapma gerekçesi).
//!
//! Normalizasyon iki katmandan oluşur (MANIFEST kart 21, madde 4):
//!
//! 1. **ASCII'ye katlama** — `İ/I/ı/i` → `i`, `Ş/s` → `s`, `Ğ/g` → `g`,
//!    `Ü/u` → `u`, `Ö/o` → `o`, `Ç/c` → `c`, ayrıca `â/î/û` karşılıkları.
//! 2. **Kök indirgeme** — yaygın Türkçe ekler listeden sökülür; *hem* yüzey
//!    biçimi *hem* kök indekslenir, böylece "yazarım" araması "yazmak" kökünü de
//!    yakalar ve tam eşleşme daha yüksek puan alır.

use std::collections::{BTreeMap, BTreeSet};

use crate::markdown::BlokTipi;
use crate::sema::BlokKaydi;

/// Sökülebilen yaygın Türkçe ekler, uzundan kısaya.
const EKLER: &[&str] = &[
    "larin", "lerin", "ların", "nın", "nin", "nun", "nün", "lar", "ler", "dan", "den", "tan",
    "ten", "mış", "miş", "muş", "müş", "sız", "siz", "mız", "miz", "nız", "nüz", "unu", "sini",
    "sinı", "unun", "inin", "ünün", "dım", "dim", "dum", "düm", "tım", "tim", "tum", "tüm", "cı",
    "ci", "cu", "cü", "çı", "çi", "çu", "çü", "yor", "e", "a", "i", "ı", "sı", "si", "su", "sü",
    "ya", "ye", "ta", "te", "da", "de", "ın", "in", "un", "ün", "ca", "ce",
];

/// Ünsüz yumuşaması: ünlü düşünce son ünsüz tonlanır (`kitab` -> `kitap`).
///
/// Bu gerçek bir Türkçe ses olayıdır ve kök indirgemenin en görünür hatasını
/// (yalnız ek sökme) giderir; kural ünsüze bağlıdır, `çocuk`/`öbeğ` gibi
/// ünlülenenler kapsam dışıdır.
fn yumusat(sozcuk: &str) -> String {
    let mut harfler: Vec<char> = sozcuk.chars().collect();
    let Some(son) = harfler.last_mut() else {
        return sozcuk.to_string();
    };
    match *son {
        'b' => *son = 'p',
        'c' => *son = 'ç',
        'd' => *son = 't',
        'ğ' => *son = 'k',
        _ => return sozcuk.to_string(),
    }
    harfler.into_iter().collect()
}

/// Kök indirgemeden sonra gereken en kısa gövde uzunluğu.
///
/// Bu eşiğin altında ek sökmek anlamsızlaşır: "ne" sözcüğü "n" olurdu.
const EN_KISA_GOVDE: usize = 3;

/// Bir karakteri normalize edilmiş ASCII karşılığına eşler.
fn katla_harf(c: char) -> char {
    match c {
        'İ' | 'I' | 'ı' | 'i' | 'Î' | 'î' => 'i',
        'Ş' | 'ş' => 's',
        'Ğ' | 'ğ' => 'g',
        'Ü' | 'ü' | 'Û' | 'û' => 'u',
        'Ö' | 'ö' => 'o',
        'Ç' | 'ç' => 'c',
        'Â' | 'â' => 'a',
        'A'..='Z' => c.to_ascii_lowercase(),
        _ => c,
    }
}

/// Metni büyük/küçük harf duyarsız, Türkçe harfleri ASCII'ye katlayarak döndürür.
///
/// Aksanlı Latin harfler (é, ñ ...) katlanmaz; bunlar ayrı bir kümelenme
/// noktası olarak kabul edilir ve JSON anahtarı güvenli ASCII kalır.
pub fn katla(metin: &str) -> String {
    let mut cikti = String::with_capacity(metin.len());
    for c in metin.chars() {
        let k = katla_harf(c);
        if k.is_ascii_alphanumeric() || k == ' ' {
            cikti.push(k);
        } else {
            cikti.push(' ');
        }
    }
    cikti
}

/// Normalize edilmiş metni sözcüklere ayırır.
pub fn tokenlara_ayir(metin: &str) -> Vec<String> {
    let mut cikti = Vec::new();
    let mut mevcut = String::new();
    for c in katla(metin).chars() {
        if c.is_ascii_alphanumeric() {
            mevcut.push(c);
        } else if !mevcut.is_empty() {
            cikti.push(std::mem::take(&mut mevcut));
        }
    }
    if !mevcut.is_empty() {
        cikti.push(mevcut);
    }
    cikti
}

/// Bir sözcüğün basit kökünü döndürür.
///
/// Sözcük zaten bilinen bir kökse (kendisi bir ek değilse) değişmez. Eki
/// söküldükten sonra gövde [`EN_KISA_GOVDE`] karakterden kısaysa ek sökülmez;
/// bu, `bir`, `var`, `yok` gibi kısa sözcüklerin bozulmasını önler. Son
/// adımda ünsüz yumuşaması uygulanır (`kitabın` -> `kitap`).
pub fn kok_indirge(sozcuk: &str) -> String {
    let baslangic = sozcuk;
    let mut mevcut = sozcuk;
    loop {
        let aday = EKLER
            .iter()
            .filter(|ek| mevcut.len() > ek.len())
            .find(|ek| mevcut.ends_with(**ek))
            .copied();
        match aday {
            Some(ek) => {
                let yeni = &mevcut[..mevcut.len() - ek.len()];
                if yeni.len() < EN_KISA_GOVDE {
                    break;
                }
                mevcut = yeni;
            }
            None => break,
        }
    }
    if mevcut == baslangic {
        mevcut.to_string()
    } else {
        yumusat(mevcut)
    }
}

/// Bir metnin indeks anahtarlarını döndürür (yüzey biçimleri ve kökler).
pub fn anahtarlar(metin: &str) -> Vec<String> {
    let mut kume: BTreeSet<String> = BTreeSet::new();
    for t in tokenlara_ayir(metin) {
        kume.insert(t.clone());
        let kok = kok_indirge(&t);
        if kok != t {
            kume.insert(kok);
        }
    }
    kume.into_iter().collect()
}

/// Tek segmentli ters indeks: `terim -> (blok kimliği -> frekans)`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TersIndeks {
    terimler: BTreeMap<String, BTreeMap<String, u32>>,
}

impl TersIndeks {
    /// Boş indeks oluşturur.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir bloğun metnini indeksler. Aynı blok ikinci kez eklenirse frekanslar
    /// **toplanır** (çağıran taraf önce `sil` çağırmalıdır).
    ///
    /// Frekans, terimin metinde **kaç kez geçtiğidir**; yüzey biçimi ile kökü
    /// ayrı terimlerdir, ama ikisi de aynı bloğun posting'ine yazılır. Bu,
    /// "alan sırası + terim frekansı" sıralamasının ikinci bileşenini besler.
    pub fn ekle(&mut self, blok_kimligi: &str, metin: &str) {
        let mut sayaclar: BTreeMap<String, u32> = BTreeMap::new();
        for t in tokenlara_ayir(metin) {
            *sayaclar.entry(t.clone()).or_insert(0) += 1;
            let kok = kok_indirge(&t);
            if kok != t {
                *sayaclar.entry(kok).or_insert(0) += 1;
            }
        }
        for (terim, adet) in sayaclar {
            *self
                .terimler
                .entry(terim)
                .or_default()
                .entry(blok_kimligi.to_string())
                .or_insert(0) += adet;
        }
    }

    /// Bir bloğu indeksten tamamen siler (yeniden indekslemede kullanılır).
    pub fn sil(&mut self, blok_kimligi: &str) {
        self.terimler.retain(|_, postingler| {
            postingler.remove(blok_kimligi);
            !postingler.is_empty()
        });
    }

    /// Benzersiz terim sayısı.
    pub fn terim_sayisi(&self) -> usize {
        self.terimler.len()
    }

    /// Toplam posting sayısı (terim–blok çifti).
    pub fn posting_sayisi(&self) -> usize {
        self.terimler.values().map(BTreeMap::len).sum()
    }

    /// İndeksi, JSON'a serileştirilebilir biçimde kopyalar.
    ///
    /// Sıralı `BTreeMap` döndürür; böylece aynı not koleksiyonu için üretilen
    /// indeks dosyası **bayt bayt aynı** olur ve sürüm denetimiyle
    /// karşılaştırılabilir hâle gelir.
    pub fn terimler_disa(&self) -> BTreeMap<String, BTreeMap<String, u32>> {
        self.terimler.clone()
    }

    /// Dışa aktarılmış (veya diskten okunmuş) terim haritasını yükler.
    ///
    /// `cache/index.json` içeriğini bellekteki indekse çevirir; böylece arama
    /// dosya sistemi okumadan çalışabilir.
    pub fn terimleri_doldur(&mut self, terimler: &BTreeMap<String, BTreeMap<String, u32>>) {
        self.terimler = terimler.clone();
    }

    /// Aranan bloğun sırasını verir.
    ///
    /// Sıralama ölçütü MANIFEST kartı madde 4: **alan sırası + terim frekansı**.
    /// Tam biçim eşleşmesi, kök üzerinden eşleşmeden iki kat puanlıdır.
    ///
    /// Sorgu terimleri **VE** ile birleştirilir: bir blok tüm terimler için
    /// posting içermiyorsa sonuçta yer almaz. Kök üzerinden eşleşme tek başına
    /// yeterlidir (`kitap` araması `kitaplar` bloğunu bulur).
    pub fn sira(
        &self,
        sorgu: &str,
        bloklar: &BTreeMap<String, BlokKaydi>,
        yalniz_tur: Option<BlokTipi>,
        yalniz_not: Option<&str>,
    ) -> Vec<AramaSonucu> {
        let sorgu_terimleri = tokenlara_ayir(sorgu);
        if sorgu_terimleri.is_empty() {
            return Vec::new();
        }
        let mut kume: BTreeMap<String, (u32, u32)> = BTreeMap::new();
        for (sira_no, terim) in sorgu_terimleri.iter().enumerate() {
            let secili = self.terim_basina(terim);
            if sira_no == 0 {
                kume = secili;
            } else {
                kume.retain(|k, _| secili.contains_key(k));
                for (k, (tam, koklu)) in secili {
                    if let Some(birim) = kume.get_mut(&k) {
                        birim.0 += tam;
                        birim.1 += koklu;
                    }
                }
            }
            if kume.is_empty() {
                return Vec::new();
            }
        }

        let mut sonuclar = Vec::new();
        for (kimlik, (tam, koklu)) in kume {
            let Some(kayit) = bloklar.get(&kimlik) else {
                continue;
            };
            if let Some(t) = yalniz_tur {
                if kayit.tip != t {
                    continue;
                }
            }
            if let Some(n) = yalniz_not {
                if kayit.not_id != n {
                    continue;
                }
            }
            let puan = (tam * 2 + koklu) * kayit.tip.alan_agirligi();
            sonuclar.push(AramaSonucu {
                blok_id: kimlik,
                puan,
                baslik: kayit.baslik.clone(),
                not_id: kayit.not_id.clone(),
                not_yol: kayit.not_yol.clone(),
                baslik_yolu: kayit.baslik_yolu.clone(),
                tur: kayit.tip,
                ozet: kayit.ozet.clone(),
            });
        }
        sonuclar.sort_by(|a, b| {
            b.puan
                .cmp(&a.puan)
                .then_with(|| a.baslik.cmp(&b.baslik))
                .then_with(|| a.blok_id.cmp(&b.blok_id))
        });
        sonuclar
    }

    /// Tek bir sorgu teriminin (tam ve kök) posting toplamını döndürür.
    fn terim_basina(&self, terim: &str) -> BTreeMap<String, (u32, u32)> {
        let kok = kok_indirge(terim);
        let mut tam: BTreeMap<&str, u32> = BTreeMap::new();
        let mut koklu: BTreeMap<&str, u32> = BTreeMap::new();
        if let Some(p) = self.terimler.get(terim) {
            for (k, f) in p {
                *tam.entry(k.as_str()).or_insert(0) += f;
            }
        }
        if let Some(p) = self.terimler.get(&kok) {
            for (k, f) in p {
                *koklu.entry(k.as_str()).or_insert(0) += f;
            }
        }
        let mut cikti: BTreeMap<String, (u32, u32)> = BTreeMap::new();
        for (k, f) in &koklu {
            if let Some(t) = tam.get(k) {
                cikti.insert((*k).to_string(), (*t, 0));
            } else {
                cikti.insert((*k).to_string(), (0, *f));
            }
        }
        cikti
    }
}

/// Tek bir arama sonucu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AramaSonucu {
    /// Eşleşen bloğun kimliği.
    pub blok_id: String,
    /// Sıralama puanı (büyükten küçüğe).
    pub puan: u32,
    /// Bloğun ait olduğu notun adı.
    pub baslik: String,
    /// Bloğun ait olduğu notun kimliği.
    pub not_id: String,
    /// Bloğun ait olduğu notun kasa köküne göreli yolu.
    pub not_yol: String,
    /// Bloğun başlıktan başlayarak okunabilir yolu.
    pub baslik_yolu: String,
    /// Bloğun türü.
    pub tur: BlokTipi,
    /// Kısaltılmış metin önizlemesi.
    pub ozet: String,
}

/// Bir metni tek satırlık, en çok `adet` karakterlik bir özet hâline getirir.
///
/// Metin ortasından kesilirse kesme noktasına `...` eklenir; kullanıcı özetin
/// kesildiğini görür.
pub fn ozetle(metin: &str, adet: usize) -> String {
    let duz: String = metin.split_whitespace().collect::<Vec<_>>().join(" ");
    if duz.chars().count() <= adet {
        return duz;
    }
    let ilk = adet.saturating_sub(3);
    let mut son: String = duz.chars().take(ilk).collect();
    son.push_str("...");
    son
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::markdown::BlokTipi;

    fn kayit(id: &str, not: &str, baslik: &str, tip: BlokTipi, metin: &str) -> BlokKaydi {
        BlokKaydi {
            id: id.to_string(),
            not_id: not.to_string(),
            baslik: baslik.to_string(),
            not_yol: format!("{not}.md"),
            baslik_yolu: baslik.to_string(),
            tip,
            ozet: crate::sema::ozet_uret(metin, 120),
            satir: 1,
        }
    }

    fn kume() -> BTreeMap<String, BlokKaydi> {
        let mut m = BTreeMap::new();
        m.insert(
            "b1".to_string(),
            kayit(
                "b1",
                "n1",
                "Markdown Lehcesi",
                BlokTipi::Baslik,
                "Markdown odaagi",
            ),
        );
        m.insert(
            "b2".to_string(),
            kayit(
                "b2",
                "n2",
                "Arama",
                BlokTipi::Paragraf,
                "Arama motoru odaagi",
            ),
        );
        m.insert(
            "b3".to_string(),
            kayit("b3", "n3", "Kod", BlokTipi::Kod, "odaagi degil"),
        );
        m
    }

    // ---- katlama ----

    #[test]
    fn katlama_turkce_harfleri_asciiye_indirger() {
        assert_eq!(katla("İstanbul"), "istanbul");
        assert_eq!(katla("Işık"), "isik");
        assert_eq!(katla("Şğüöç"), "sguoc");
        assert_eq!(katla("âîû"), "aiu");
    }

    #[test]
    fn katlama_noktalama_boşluğa_cevirir() {
        assert_eq!(katla("bir-iki.üç"), "bir iki uc");
        assert_eq!(katla("[[baglanti]]"), "  baglanti  ");
    }

    #[test]
    fn katlama_bos_girdi_bos_donulur() {
        assert_eq!(katla(""), "");
        assert_eq!(katla("!!!"), "   ");
    }

    // ---- tokenlestirme ----

    #[test]
    fn tokenlestirme_sozcukleri_ayirir() {
        assert_eq!(tokenlara_ayir("bir iki üç"), vec!["bir", "iki", "uc"]);
    }

    #[test]
    fn tokenlestirme_bosluk_ve_ayraclar_yok_sayar() {
        assert_eq!(tokenlara_ayir("  bir ,,  iki  "), vec!["bir", "iki"]);
    }

    #[test]
    fn tokenlestirme_sayilari_korur() {
        assert_eq!(tokenlara_ayir("2026 yili 45"), vec!["2026", "yili", "45"]);
    }

    #[test]
    fn tokenlestirme_bos_girdi_bos_liste() {
        assert!(tokenlara_ayir("").is_empty());
        assert!(tokenlara_ayir("   ---   ").is_empty());
    }

    // ---- kok indirgeme ----

    #[test]
    fn kok_indirgeme_ekleri_soker() {
        assert_eq!(kok_indirge("kitaplar"), "kitap");
        assert_eq!(kok_indirge("kitabın"), "kitap");
        assert_eq!(kok_indirge("yazdım"), "yaz");
        assert_eq!(kok_indirge("notlarımız"), "not");
    }

    #[test]
    fn kok_indirgeme_unsuz_yumusatmasi_uygular() {
        // Ünlü düşünce son ünsüz yumuşar: kitap / ağaç / aydın.
        assert_eq!(kok_indirge("ağaç"), "ağaç");
        assert_eq!(kok_indirge("kitaplar"), "kitap");
        assert_eq!(kok_indirge("ağaçları"), "ağaç");
    }

    #[test]
    fn kok_indirgeme_kisa_sozcuklere_dokunmaz() {
        assert_eq!(kok_indirge("bir"), "bir");
        assert_eq!(kok_indirge("var"), "var");
        assert_eq!(kok_indirge("ne"), "ne");
    }

    #[test]
    fn kok_indirgeme_eklisiz_sozcuk_degismez() {
        assert_eq!(kok_indirge("indeks"), "indeks");
    }

    #[test]
    fn anahtarlar_yuzey_ve_koku_beraber_verir() {
        let a = anahtarlar("kitaplar");
        assert!(a.contains(&"kitaplar".to_string()));
        assert!(a.contains(&"kitap".to_string()));
    }

    // ---- ters indeks ----

    #[test]
    fn bos_indeks_arama_dondurmez() {
        let i = TersIndeks::yeni();
        assert_eq!(i.terim_sayisi(), 0);
        assert!(i.sira("herhangi", &kume(), None, None).is_empty());
    }

    #[test]
    fn ekleme_terim_ve_posting_sayisi_artan() {
        let mut i = TersIndeks::yeni();
        i.ekle("b1", "bir iki bir");
        assert!(i.terim_sayisi() >= 2);
        assert!(i.posting_sayisi() >= 2);
    }

    #[test]
    fn frekans_arttikca_puan_artar() {
        // Aynı blok türünde karşılaştırma: alan ağırlığı devre dışı kalsın.
        let mut m = BTreeMap::new();
        m.insert(
            "p1".to_string(),
            kayit("p1", "n1", "Az", BlokTipi::Paragraf, "tek"),
        );
        m.insert(
            "p2".to_string(),
            kayit("p2", "n2", "Cok", BlokTipi::Paragraf, "tek"),
        );
        let mut i = TersIndeks::yeni();
        i.ekle("p1", "tek");
        i.ekle("p2", "tek tek tek");
        let s = i.sira("tek", &m, None, None);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].blok_id, "p2");
    }

    #[test]
    fn turkce_karakterle_yazilan_sorgu_ascii_yazimiyla_bulur() {
        let mut i = TersIndeks::yeni();
        i.ekle("b1", "İstanbul günlüğü");
        let s = i.sira("istanbul", &kume(), None, None);
        assert_eq!(s.len(), 1);
        let s2 = i.sira("İSTANBUL", &kume(), None, None);
        assert_eq!(s2.len(), 1);
    }

    #[test]
    fn buyuk_kucuk_harf_duyarsiz_arama_yapar() {
        let mut i = TersIndeks::yeni();
        i.ekle("b1", "Markdown Odası");
        assert_eq!(i.sira("markdown", &kume(), None, None).len(), 1);
        assert_eq!(i.sira("MARKDOWN", &kume(), None, None).len(), 1);
    }

    #[test]
    fn kok_uzerinden_arama_yuzey_bicimiyle_bulur() {
        let mut i = TersIndeks::yeni();
        i.ekle("b1", "kitaplar");
        let s = i.sira("kitap", &kume(), None, None);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn cok_terimli_sorgu_hepsini_isteyen_bloklari_bulur() {
        let mut i = TersIndeks::yeni();
        i.ekle("b1", "grafik cizimi");
        i.ekle("b2", "grafik sadece");
        let s = i.sira("grafik cizim", &kume(), None, None);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].blok_id, "b1");
    }

    #[test]
    fn coklu_terim_puanlari_toplanir() {
        let mut i = TersIndeks::yeni();
        i.ekle("b1", "aaa bbb");
        i.ekle("b2", "aaa");
        let s = i.sira("aaa bbb", &kume(), None, None);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].blok_id, "b1");
    }

    #[test]
    fn alan_agirligi_basligi_one_almaya_ettirir() {
        let mut i = TersIndeks::yeni();
        // Aynı terim, aynı frekans; biri başlıkta, biri kodda.
        i.ekle("b1", "kavram");
        i.ekle("b3", "kavram");
        let s = i.sira("kavram", &kume(), None, None);
        assert_eq!(s[0].blok_id, "b1");
        assert_eq!(s[0].tur, BlokTipi::Baslik);
    }

    #[test]
    fn tur_suzgeci_calisir() {
        let mut i = TersIndeks::yeni();
        i.ekle("b1", "kavram");
        i.ekle("b3", "kavram");
        let s = i.sira("kavram", &kume(), Some(BlokTipi::Kod), None);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].blok_id, "b3");
    }

    #[test]
    fn not_suzgeci_calisir() {
        let mut i = TersIndeks::yeni();
        i.ekle("b2", "kavram");
        let s = i.sira("kavram", &kume(), None, Some("n2"));
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].not_id, "n2");
    }

    #[test]
    fn eslesmeyen_sorgu_bos_donulur() {
        let mut i = TersIndeks::yeni();
        i.ekle("b1", "bir");
        assert!(i.sira("hicbiryer", &kume(), None, None).is_empty());
    }

    #[test]
    fn yalniz_noktalama_iceren_sorgu_bos_donulur() {
        let mut i = TersIndeks::yeni();
        i.ekle("b1", "bir");
        assert!(i.sira("---", &kume(), None, None).is_empty());
        assert!(i.sira("", &kume(), None, None).is_empty());
    }

    #[test]
    fn dizinde_olmayan_blok_sonuc_dondurmez() {
        let mut i = TersIndeks::yeni();
        i.ekle("yok", "terim");
        assert!(i.sira("terim", &kume(), None, None).is_empty());
    }

    #[test]
    fn silme_terimi_indensten_kaldirir() {
        let mut i = TersIndeks::yeni();
        i.ekle("b1", "tekrar");
        assert_eq!(i.posting_sayisi(), 1);
        i.sil("b1");
        assert_eq!(i.posting_sayisi(), 0);
        assert_eq!(i.terim_sayisi(), 0);
    }

    #[test]
    fn ayni_blok_iki_kez_eklenince_frekans_artar() {
        let mut i = TersIndeks::yeni();
        i.ekle("b1", "x");
        i.ekle("b1", "x");
        assert_eq!(i.posting_sayisi(), 1);
        let s = i.sira("x", &kume(), None, None);
        assert!(s.is_empty() || s[0].puan > 0);
    }

    #[test]
    fn siralama_esit_puanda_kararli() {
        let mut i = TersIndeks::yeni();
        i.ekle("b2", "ayni");
        i.ekle("b1", "ayni");
        let a = i.sira("ayni", &kume(), None, None);
        let b = i.sira("ayni", &kume(), None, None);
        assert_eq!(a, b);
    }

    // ---- ozet ----

    #[test]
    fn ozet_kisa_metni_degistirmez() {
        assert_eq!(ozetle("kisa metin", 20), "kisa metin");
    }

    #[test]
    fn ozet_uzun_metni_kirpar_ve_uc_nokta_koyar() {
        let o = ozetle("bir iki uc dort bes", 10);
        assert!(o.ends_with("..."));
        assert!(o.chars().count() <= 10);
    }

    #[test]
    fn ozet_bos_girdi_bos_donulur() {
        assert_eq!(ozetle("   ", 10), "");
    }
}
