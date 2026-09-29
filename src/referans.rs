//! Bağlantı çıkarma ve referans çözümleme.
//!
//! İki bağlantı lehçesi tanınır (rapor b05, "Referans çözümleme"):
//!
//! * `[[not adı]]` — Obsidian tarzı, **not adına** gider.
//! * `((blok kimligi))` — Logseq tarzı, **blok kimliğine** gider.
//!
//! `[[ad|etiket]]` biçimi de kabul edilir; etiket kısımı çözümlemede yok sayılır
//! (Obsidian uyumluluğu için).
//!
//! Çözülemeyen referanslar **sessizce yutulmaz**: her bağlantı üç durumdan birine
//! düşer — `Cozuldu`, `Belirsiz` (yinelenen ad) veya `Kirik` — ve `check`
//! alt komutunda ayrı listeler hâlinde yazılır.

use std::collections::BTreeMap;

/// Bir bağlantının lehçesini belirleyen tür.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BaglantiTipi {
    /// `[[not adı]]` — not adına giden bağlantı.
    NotAdi,
    /// `((blok kimligi))` — blok kimliğine giden bağlantı.
    BlokKimligi,
}

impl BaglantiTipi {
    /// Lehçenin açıklama metni (`check` çıktısı için).
    pub fn aciklama(self) -> &'static str {
        match self {
            BaglantiTipi::NotAdi => "[[not adi]]",
            BaglantiTipi::BlokKimligi => "((blok kimligi))",
        }
    }
}

/// Bir metinden çıkarılmış ham bağlantı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Baglanti {
    /// Bağlantının lehçesi.
    pub tip: BaglantiTipi,
    /// Çözümlemede kullanılacak hedef metni (etiket temizlenmiş).
    pub hedef: String,
    /// Kaynak metinde bulunan özgün biçim.
    pub ham: String,
    /// 1 tabanlı satır numarası.
    pub satir: usize,
}

/// Metindeki tüm `[[...]]` ve `((...))` bağlantılarını sırayla çıkarır.
///
/// Tarama satır sınırları içinde yapılır: bir `[[` karşılığı olmayan bir `]]`
/// veya satır sonunu aşan bir açılış işareti bağlantı sayılmaz.
///
/// Neden `char` dizisi üzerinden tarama: bayt bayt ilerlemek çok baytlı
/// (Türkçe) karakterlerin ortasında durabilir ve `&metin[i..]` dilimlemesi
/// panik yapardı. `char` üzerinden ilerlemek bu riski kökten kaldırır.
pub fn baglantilari_cikar(metin: &str) -> Vec<Baglanti> {
    let karakterler: Vec<char> = metin.chars().collect();
    let mut cikti = Vec::new();
    let mut i = 0usize;
    let mut satir = 1usize;
    while i < karakterler.len() {
        if karakterler[i] == '\n' {
            satir += 1;
            i += 1;
            continue;
        }
        let tip = if karakterler[i..].starts_with(&['[', '[']) {
            BaglantiTipi::NotAdi
        } else if karakterler[i..].starts_with(&['(', '(']) {
            BaglantiTipi::BlokKimligi
        } else {
            i += 1;
            continue;
        };
        let kapanis = match tip {
            BaglantiTipi::NotAdi => ']',
            BaglantiTipi::BlokKimligi => ')',
        };
        let govde_baslangic = i + 2;
        let kalan = &karakterler[govde_baslangic..];
        let kapi = kalan.iter().position(|c| *c == '\n').unwrap_or(kalan.len());
        match kalan[..kapi].iter().position(|c| *c == kapanis) {
            Some(ofset) => {
                let kapanis_indeks = govde_baslangic + ofset;
                if karakterler[kapanis_indeks + 1..].starts_with(&[kapanis]) {
                    let govde: String = karakterler[govde_baslangic..kapanis_indeks]
                        .iter()
                        .collect();
                    let ham: String = karakterler[i..kapanis_indeks + 2].iter().collect();
                    cikti.push(Baglanti {
                        tip,
                        hedef: etiketi_at(&govde).to_string(),
                        ham,
                        satir,
                    });
                    i = kapanis_indeks + 2;
                } else {
                    i += 2;
                }
            }
            None => i += 2,
        }
    }
    cikti
}

/// `ad|etiket` biçimindeki gövdeden yalnızca ad kısmını alır ve kırpar.
fn etiketi_at(govde: &str) -> &str {
    match govde.split_once('|') {
        Some((ad, _)) => ad.trim(),
        None => govde.trim(),
    }
}

/// Çözümleme için gereken, not ve blok dizinleri.
#[derive(Debug, Default, Clone)]
pub struct Duzce {
    /// Not adı (ASCII'ye katlanmış) → not kimlikleri. Yinelenen adlar burada
    /// birden çok giriş taşır ve **belirsizlik** olarak bildirilir.
    not_adi: BTreeMap<String, Vec<String>>,
    /// Blok kimliği → (not kimliği, blok metni, satır).
    blok: BTreeMap<String, BlokKaydi>,
}

/// Düzceye eklenen bir blok kaydı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlokKaydi {
    /// Bloğun ait olduğu notun kimliği.
    pub not_id: String,
    /// Bloğun okunabilir başlık yolu (`"Giris > Ayrinti"`).
    pub baslik_yolu: String,
    /// Bloğun düz metni.
    pub metin: String,
    /// Bloğun satır numarası.
    pub satir: usize,
}

impl BlokKaydi {
    /// Yeni bir blok kaydı oluşturur.
    pub fn yeni(not_id: &str, baslik_yolu: &str, metin: &str, satir: usize) -> Self {
        Self {
            not_id: not_id.to_string(),
            baslik_yolu: baslik_yolu.to_string(),
            metin: metin.to_string(),
            satir,
        }
    }
}

impl Duzce {
    /// Boş bir düzce oluşturur.
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir notu düzceye ekler. `ad`, notun kullanıcıya görünen adıdır.
    pub fn not_ekle(&mut self, ad: &str, not_id: &str) {
        let anahtar = crate::indeks::katla(ad);
        self.not_adi
            .entry(anahtar)
            .or_default()
            .push(not_id.to_string());
    }

    /// Bir bloğu düzceye ekler.
    pub fn blok_ekle(&mut self, kimlik: &str, kaydi: BlokKaydi) {
        self.blok.insert(kimlik.to_string(), kaydi);
    }

    /// Bir not adının kaç nota karşılık geldiğini döndürür.
    pub fn not_aday_sayisi(&self, ad: &str) -> usize {
        self.not_adi
            .get(&crate::indeks::katla(ad))
            .map(Vec::len)
            .unwrap_or(0)
    }

    /// Ad ile eşleşen not kimliklerini döndürür.
    pub fn not_adaylari(&self, ad: &str) -> &[String] {
        self.not_adi
            .get(&crate::indeks::katla(ad))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Blok kimliğiyle kayıt arar.
    pub fn blok_ara(&self, kimlik: &str) -> Option<&BlokKaydi> {
        self.blok.get(kimlik)
    }

    /// Düzceye eklenen blok sayısını döndürür.
    pub fn blok_sayisi(&self) -> usize {
        self.blok.len()
    }
}

/// Bir bağlantının çözüm durumu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Durum {
    /// Hedef bulundu.
    Cozuldu {
        /// Hedefin türü.
        hedef: HedefTuru,
    },
    /// Hedef adı birden çok nota karşılık geliyor.
    Belirsiz {
        /// Aday not kimlikleri (sıralı).
        adaylar: Vec<String>,
    },
    /// Hedef bulunamadı.
    Kirik,
}

/// Çözülen hedefin türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HedefTuru {
    /// Bir nota çözüldü.
    Not,
    /// Bir bloğa çözüldü.
    Blok,
}

/// Çözümlenmiş tek bir bağlantı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cozum {
    /// Kaynak bağlantının lehçesi.
    pub tip: BaglantiTipi,
    /// Ham hedef metni.
    pub hedef: String,
    /// Bağlantının bulunduğu not kimliği.
    pub kaynak_not: String,
    /// Bağlantının bulunduğu blok kimliği.
    pub kaynak_blok: String,
    /// 1 tabanlı satır numarası.
    pub satir: usize,
    /// Çözüm durumu.
    pub durum: Durum,
}

impl Cozum {
    /// Bu bağlantının kırık olup olmadığını döndürür.
    pub fn kirik_mi(&self) -> bool {
        matches!(self.durum, Durum::Kirik)
    }

    /// Bu bağlantının belirsiz olup olmadığını döndürür.
    pub fn belirsiz_mi(&self) -> bool {
        matches!(self.durum, Durum::Belirsiz { .. })
    }
}

/// `check` alt komutunun döndürdüğü toplu rapor.
#[derive(Debug, Default, Clone)]
pub struct Rapor {
    /// Çözülen bağlantılar.
    pub cozulen: Vec<Cozum>,
    /// Belirsiz (yinelenen ad) bağlantılar.
    pub belirsiz: Vec<Cozum>,
    /// Kırık bağlantılar.
    pub kirik: Vec<Cozum>,
    /// Kendine bağlantı veren bloklar.
    pub kendine: Vec<Cozum>,
}

impl Rapor {
    /// Rapora bir çözüm ekler; duruma göre doğru listeye yerleştirir.
    pub fn ekle(&mut self, cozum: Cozum) {
        if cozum.kirik_mi() {
            self.kirik.push(cozum);
        } else if cozum.belirsiz_mi() {
            self.belirsiz.push(cozum);
        } else {
            self.cozulen.push(cozum);
        }
    }

    /// Raporda kayıtlı toplam bağlantı sayısı.
    pub fn toplam(&self) -> usize {
        self.cozulen.len() + self.belirsiz.len() + self.kirik.len()
    }

    /// Kırık ya da belirsiz bağlantı varsa `true`.
    pub fn sorunlu_mu(&self) -> bool {
        !self.kirik.is_empty() || !self.belirsiz.is_empty()
    }
}

/// Tek bir bağlantıyı düzceye karşı çözer.
pub fn coz(baglanti: &Baglanti, kaynak_not: &str, kaynak_blok: &str, duzce: &Duzce) -> Cozum {
    let durum = match baglanti.tip {
        BaglantiTipi::NotAdi => {
            let adaylar = duzce.not_adaylari(&baglanti.hedef);
            match adaylar.len() {
                0 => Durum::Kirik,
                1 => Durum::Cozuldu {
                    hedef: HedefTuru::Not,
                },
                _ => Durum::Belirsiz {
                    adaylar: adaylar.to_vec(),
                },
            }
        }
        BaglantiTipi::BlokKimligi => {
            if duzce.blok_ara(&baglanti.hedef).is_some() {
                Durum::Cozuldu {
                    hedef: HedefTuru::Blok,
                }
            } else {
                Durum::Kirik
            }
        }
    };
    Cozum {
        tip: baglanti.tip,
        hedef: baglanti.hedef.clone(),
        kaynak_not: kaynak_not.to_string(),
        kaynak_blok: kaynak_blok.to_string(),
        satir: baglanti.satir,
        durum,
    }
}

/// Bir bloğun kendine bağlantı verip vermediğini belirler.
///
/// * `[[kendi notu]]` — bloğun ait olduğu nota kendi bağlantısı (kendine).
/// * `((kendi kimligi))` — bloğun kendi kimliğine bağlantı (kendine).
///
/// Kapsam döngüsü **değildir**: `A -> B -> A` bir döngüdür ama kendine bağlantı
/// değildir; `graph` çıktısında döngü olarak gösterilir.
pub fn kendine_mi(cozum: &Cozum, duzce: &Duzce) -> bool {
    if cozum.kirik_mi() || cozum.belirsiz_mi() {
        return false;
    }
    match cozum.tip {
        BaglantiTipi::BlokKimligi => cozum.hedef == cozum.kaynak_blok,
        BaglantiTipi::NotAdi => {
            let adaylar = duzce.not_adaylari(&cozum.hedef);
            adaylar.len() == 1 && adaylar[0] == cozum.kaynak_not
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn hedefler(metin: &str) -> Vec<(BaglantiTipi, String)> {
        baglantilari_cikar(metin)
            .into_iter()
            .map(|b| (b.tip, b.hedef))
            .collect()
    }

    // ---- cikarim ----

    #[test]
    fn kenar_durumu_bos_girdide_bos_liste() {
        assert!(baglantilari_cikar("").is_empty());
    }

    #[test]
    fn tek_cift_koseli_parantez_cikarilir() {
        assert_eq!(
            hedefler("Sirasiyla [[not]] ve ((abc)) bagli."),
            vec![
                (BaglantiTipi::NotAdi, "not".to_string()),
                (BaglantiTipi::BlokKimligi, "abc".to_string())
            ]
        );
    }

    #[test]
    fn ayni_satirda_coklu_baglanti_cikarilir() {
        let l = baglantilari_cikar("[[a]] [[b]] [[c]]");
        assert_eq!(l.len(), 3);
        assert_eq!(l[1].hedef, "b");
    }

    #[test]
    fn etiketli_baglantida_ad_kismi_alinir() {
        assert_eq!(
            hedefler("[[not|etiket]]"),
            vec![(BaglantiTipi::NotAdi, "not".to_string())]
        );
    }

    #[test]
    fn etiket_kisiminda_ikinci_boru_yok_sayilir() {
        assert_eq!(
            hedefler("[[a|b|c]]"),
            vec![(BaglantiTipi::NotAdi, "a".to_string())]
        );
    }

    #[test]
    fn kirpma_yapilir() {
        assert_eq!(
            hedefler("[[  bosluklu  ]]"),
            vec![(BaglantiTipi::NotAdi, "bosluklu".to_string())]
        );
    }

    #[test]
    fn satir_numarasi_bulunur() {
        let l = baglantilari_cikar("bir\niki\nuc [[x]]");
        assert_eq!(l[0].satir, 3);
    }

    #[test]
    fn coklu_satirda_satirlar_ayri_sayilir() {
        let l = baglantilari_cikar("[[a]]\n\nsatir2 [[b]]\n[[c]]");
        assert_eq!(l.len(), 3);
        assert_eq!(l[1].satir, 3);
        assert_eq!(l[2].satir, 4);
    }

    #[test]
    fn kapanmayan_isaretci_baglanti_uretmez() {
        assert!(baglantilari_cikar("[[acik").is_empty());
        assert!(baglantilari_cikar("((acik").is_empty());
        assert!(baglantilari_cikar("]kapanik]").is_empty());
    }

    #[test]
    fn satir_kacisi_olan_isaretci_baglanti_uretmez() {
        assert!(baglantilari_cikar("[[acik\nkapandi]]").is_empty());
    }

    #[test]
    fn turkce_karakterli_hedef_korunur() {
        assert_eq!(
            hedefler("[[Çalışma Günü]]"),
            vec![(BaglantiTipi::NotAdi, "Çalışma Günü".to_string())]
        );
    }

    #[test]
    fn ham_bicim_korunur() {
        let l = baglantilari_cikar("metin [[ad|et]] sonu");
        assert_eq!(l[0].ham, "[[ad|et]]");
    }

    #[test]
    fn kod_ici_isaretci_de_cikarilir_ama_kapsam_disinda_birakilir() {
        // Not: v1 lehçesinde kod bloğu içindeki bağlantılar da sayılır; bu
        // davranış belgelenmiştir ve kırık referans raporunda görünür.
        let l = baglantilari_cikar("```\n[[kod ici]]\n```");
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].hedef, "kod ici");
    }

    // ---- duzce ----

    fn dolu_duzce() -> Duzce {
        let mut d = Duzce::yeni();
        d.not_ekle("Not", "n1");
        d.not_ekle("not", "n2");
        d.not_ekle("Tekil", "n3");
        d.blok_ekle("abc", BlokKaydi::yeni("n3", "B > C", "icerik", 7));
        d
    }

    #[test]
    fn duzce_adet_sayimi_yinelenen_adi_sayar() {
        let d = dolu_duzce();
        assert_eq!(d.not_aday_sayisi("Not"), 2);
        assert_eq!(d.not_aday_sayisi("Tekil"), 1);
        assert_eq!(d.not_aday_sayisi("Yok"), 0);
    }

    #[test]
    fn duzce_ad_ayrimi_buyuk_kucuk_harf_duyarsizdir() {
        let d = dolu_duzce();
        assert_eq!(d.not_adaylari("not").len(), 2);
    }

    #[test]
    fn duzce_turkce_harf_katlamasi_yapar() {
        let mut d = Duzce::yeni();
        d.not_ekle("Çalışma", "n1");
        assert_eq!(d.not_aday_sayisi("calisma"), 1);
        assert_eq!(d.not_aday_sayisi("Çalışma"), 1);
    }

    #[test]
    fn duzce_blok_arama_calisir() {
        let d = dolu_duzce();
        let k = d.blok_ara("abc");
        assert!(k.is_some());
        assert_eq!(k.unwrap().satir, 7);
        assert_eq!(d.blok_sayisi(), 1);
        assert!(d.blok_ara("yok").is_none());
    }

    // ---- cozumleme ----

    #[test]
    fn cozulen_not_baglantisi_bulunur() {
        let d = dolu_duzce();
        let b = &baglantilari_cikar("[[Tekil]]")[0];
        assert_eq!(
            coz(b, "n1", "b1", &d).durum,
            Durum::Cozuldu {
                hedef: HedefTuru::Not
            }
        );
    }

    #[test]
    fn cozulen_blok_baglantisi_bulunur() {
        let d = dolu_duzce();
        let b = &baglantilari_cikar("((abc))")[0];
        assert_eq!(
            coz(b, "n1", "b1", &d).durum,
            Durum::Cozuldu {
                hedef: HedefTuru::Blok
            }
        );
    }

    #[test]
    fn yinelenen_ad_belirsiz_isaretlenir() {
        let d = dolu_duzce();
        let b = &baglantilari_cikar("[[Not]]")[0];
        let c = coz(b, "n1", "b1", &d);
        assert!(c.belirsiz_mi());
        match c.durum {
            Durum::Belirsiz { ref adaylar } => {
                assert_eq!(adaylar.len(), 2);
                assert!(adaylar.contains(&"n1".to_string()));
            }
            _ => panic!("belirsiz olmali"),
        }
    }

    #[test]
    fn kirik_not_baglantisi_isaretlenir() {
        let d = dolu_duzce();
        let b = &baglantilari_cikar("[[Olmayan]]")[0];
        assert!(coz(b, "n1", "b1", &d).kirik_mi());
    }

    #[test]
    fn kirik_blok_baglantisi_isaretlenir() {
        let d = dolu_duzce();
        let b = &baglantilari_cikar("((yok))")[0];
        assert!(coz(b, "n1", "b1", &d).kirik_mi());
    }

    #[test]
    fn bos_koseli_parantez_kirik_baglanti_uretir() {
        let d = dolu_duzce();
        let b = &baglantilari_cikar("[[]]")[0];
        assert!(coz(b, "n1", "b1", &d).kirik_mi());
    }

    #[test]
    fn turkce_karakterli_hedef_cozulur() {
        let mut d = Duzce::yeni();
        d.not_ekle("Günlük", "n1");
        let b = &baglantilari_cikar("[[Günlük]]")[0];
        assert!(!coz(b, "n2", "b1", &d).kirik_mi());
    }

    #[test]
    fn cozum_kaynak_bilgisini_tasir() {
        let d = dolu_duzce();
        let b = &baglantilari_cikar("[[Tekil]]")[0];
        let c = coz(b, "n9", "blok9", &d);
        assert_eq!(c.kaynak_not, "n9");
        assert_eq!(c.kaynak_blok, "blok9");
        assert_eq!(c.satir, 1);
    }

    // ---- kendine baglanti ----

    #[test]
    fn kendine_blok_baglantisi_yakalnir() {
        let mut d = Duzce::yeni();
        d.not_ekle("N", "n1");
        d.blok_ekle("abc", BlokKaydi::yeni("n1", "", "m", 1));
        let b = &baglantilari_cikar("((abc))")[0];
        let c = coz(b, "n1", "abc", &d);
        assert!(kendine_mi(&c, &d));
    }

    #[test]
    fn kendine_not_baglantisi_yakalnir() {
        let d = dolu_duzce();
        let b = &baglantilari_cikar("[[Tekil]]")[0];
        let c = coz(b, "n3", "b1", &d);
        assert!(kendine_mi(&c, &d));
        let diger = coz(b, "n1", "b1", &d);
        assert!(!kendine_mi(&diger, &d));
    }

    #[test]
    fn kirk_ve_belirsiz_baglanti_kendine_sayilmaz() {
        let d = dolu_duzce();
        let b = &baglantilari_cikar("[[Olmayan]]")[0];
        assert!(!kendine_mi(&coz(b, "n1", "b1", &d), &d));
        let b2 = &baglantilari_cikar("[[Not]]")[0];
        assert!(!kendine_mi(&coz(b2, "n1", "b1", &d), &d));
    }

    // ---- rapor ----

    #[test]
    fn rapor_durumlari_ayri_listelere_koyar() {
        let d = dolu_duzce();
        let mut r = Rapor::default();
        for b in baglantilari_cikar("[[Tekil]] [[Not]] [[Yok]] ((abc))") {
            r.ekle(coz(&b, "n1", "b1", &d));
        }
        assert_eq!(r.cozulen.len(), 2);
        assert_eq!(r.belirsiz.len(), 1);
        assert_eq!(r.kirik.len(), 1);
        assert_eq!(r.toplam(), 4);
        assert!(r.sorunlu_mu());
    }

    #[test]
    fn bos_rapor_sorunlu_degildir() {
        let r = Rapor::default();
        assert_eq!(r.toplam(), 0);
        assert!(!r.sorunlu_mu());
    }

    #[test]
    fn lehce_aciklamalari_yazilir() {
        assert_eq!(BaglantiTipi::NotAdi.aciklama(), "[[not adi]]");
        assert_eq!(BaglantiTipi::BlokKimligi.aciklama(), "((blok kimligi))");
    }

    #[test]
    fn blok_kaydi_olusturucusu_alanlari_doldurur() {
        let k = BlokKaydi::yeni("n1", "A > B", "metin", 3);
        assert_eq!(k.not_id, "n1");
        assert_eq!(k.baslik_yolu, "A > B");
        assert_eq!(k.metin, "metin");
        assert_eq!(k.satir, 3);
    }
}
