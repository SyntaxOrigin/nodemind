//! Kendi Markdown ayrıştırıcımız: düz metni bir **blok ağacına** çevirir.
//!
//! Kapsam (rapor b05, MVP satırı "Markdown çekirdeği ve blok kimliği"):
//! başlıklar (`#`..`######`), listeler (iç içe madde listeleri), alıntılar (`>`),
//! kod blokları (```` ``` ````) ve paragraflar. Tablolar, satır kırılmaları,
//! satır içi biçimlendirme ve HTML bloğu **desteklenmez**; bunlar
//! "Bilinen Sınırlamalar" bölümünde belgelenmiştir.
//!
//! Sorumluluğu sözcüklerini ve bağlantı işaretçilerini (`[[x]]`, `((x))`)
//! **ayırmamaktır**: madde metni olduğu gibi saklanır, bağlantılar
//! [`crate::referans`] modülünde taranır. Bu ayrım, `[[` yazan kullanıcı ile
//! `[[` yazmayı örnekleyen kullanıcı arasındaki farkı birleştirmeyi engeller.
//!
//! Ağaç kuralları (hepsi deterministik):
//!
//! * Başlıklar kendi seviyelerine göre iç içe girer (`##` bir `#` altına girer).
//! * Liste maddeleri **her başlıktan derindir**; girinti iki boşluk = bir kademe.
//! * Başlığın hemen ardından gelen düz satırlar o başlığın çocuğudur.
//! * Bir madde veya başlığın devam satırları (girintisi eşikten büyük düz
//!   satırlar) o bloğun **çocuğu** olur; girintisi küçük satırlar kardeş kalır.
//! * Alıntı satırları `>` işaretçileri soyulur ve **yeniden ayrıştırılır**;
//!   böylece `> - madde` doğru biçimde bir liste üretir.

use serde::{Deserialize, Serialize};

use crate::kimlik::blok_kimligi;

/// Bir bloğun türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlokTipi {
    /// `#` ile başlayan başlık.
    Baslik,
    /// `-`, `*`, `+`, `12.` veya `12)` ile başlayan liste maddesi.
    Liste,
    /// `>` ile başlayan alıntı.
    Alinti,
    /// ```` ``` ```` arasında kalan kod bloğu.
    Kod,
    /// Diğer tüm düz metin.
    Paragraf,
}

impl BlokTipi {
    /// Türün ASCII etiketini döndürür (kimlik girdisi ve çıktı için).
    pub fn etiket(self) -> &'static str {
        match self {
            BlokTipi::Baslik => "baslik",
            BlokTipi::Liste => "madde",
            BlokTipi::Alinti => "alinti",
            BlokTipi::Kod => "kod",
            BlokTipi::Paragraf => "paragraf",
        }
    }

    /// Arama sonucu sıralamasında kullanılan alan ağırlığı.
    ///
    /// Sıralama ölçütü MANIFEST kartı madde 4'te "alan sırası + terim frekansı"
    /// olarak tanımlanmıştır: başlık en güçlü, kod bloğu en zayıfı sinyaldir.
    pub fn alan_agirligi(self) -> u32 {
        match self {
            BlokTipi::Baslik => 40,
            BlokTipi::Liste => 30,
            BlokTipi::Alinti => 20,
            BlokTipi::Paragraf => 10,
            BlokTipi::Kod => 5,
        }
    }
}

/// Ayrıştırılmış tek bir blok (ağacın bir düğümü).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blok {
    /// Kararlı blok kimliği ([`crate::kimlik::blok_kimligi`]).
    pub kimlik: String,
    /// Blok türü.
    pub tip: BlokTipi,
    /// Not ağacındaki konum yolu (`"0"`, `"0.1"`, ...).
    pub yol: String,
    /// Bloğun düz metni (işaretçi ve bağlantı sözdizimi korunur).
    pub metin: String,
    /// Bloğun kendi ağaç derinliği (0 = not kökü).
    pub derinlik: u8,
    /// Bloğun bulunduğu 1 tabanlı satır numarası.
    pub baslangic_satir: usize,
    /// Alt bloklar.
    pub cocuklar: Vec<Blok>,
}

/// Tek bir satırın sınıflandırılması.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SatirTuru {
    /// Yalnızca boşluk içeren satır.
    Bos,
    /// Kod bloğu çiti (```` ``` ```` veya `~~~`). Açma/kapama kararı ayrıştırıcının
    /// durumuna göre verilir; bu yüzden ayrım burada yapılmaz.
    Kod {
        /// Sondaki çit sayısı (açma ve kapama eşleşmeli).
        isaret: u8,
        /// Boşsa kod dili etiketi yoktur.
        dil: String,
    },
    /// ATX başlık.
    Baslik {
        /// Başlık seviyesi (1..=6).
        seviye: u8,
        /// Başlık metni (`#` ve sondaki `#` dizisi temizlenmiş).
        metin: String,
    },
    /// Liste maddesi.
    Liste {
        /// Satırın bayt cinsinden girintisi.
        girinti: usize,
        /// Madde metni.
        metin: String,
    },
    /// Alıntı satırı.
    Alinti {
        /// Ardışık `>` sayısı (1 = sığ alıntı).
        derinlik: u8,
        /// `>` işaretçileri ve bir boşluk temizlenmiş içerik.
        icerik: String,
    },
    /// Düz metin satırı.
    Duz {
        /// Satırın bayt cinsinden girintisi.
        girinti: usize,
        /// Kırpılmış metin.
        metin: String,
    },
}

/// Bir satırın başındaki kod çitini tanır.
fn kod_cesiti(kirp: &str) -> Option<(u8, String)> {
    let ilk = kirp.chars().next()?;
    if ilk != '`' && ilk != '~' {
        return None;
    }
    let isaret = kirp.chars().take_while(|c| *c == ilk).count();
    if isaret < 3 {
        return None;
    }
    let kalan = &kirp[isaret..];
    // CommonMark: kapanış çitinde bilgi etiketi olamaz.
    if ilk == '`' && kalan.contains('`') {
        return None;
    }
    Some((isaret as u8, kalan.trim().to_string()))
}

/// Bir Markdown satırını sınıflandırır ve ilgili bileşenleri ayırır.
///
/// Dışarıdan bağımsız, saf bir fonksiyondur; birim testleri doğrudan bunu
/// doğrulayabilir.
pub fn parcala_satir(satir: &str) -> SatirTuru {
    let s = satir.trim_end_matches('\r');
    let kirp = s.trim();
    if kirp.is_empty() {
        return SatirTuru::Bos;
    }
    let girinti = s.len() - s.trim_start_matches(' ').len();

    if let Some((isaret, dil)) = kod_cesiti(kirp) {
        return SatirTuru::Kod { isaret, dil };
    }

    // Alıntı: `>` sayısı işaretçi sayısını verir.
    let isaret_sayisi = kirp.chars().take_while(|c| *c == '>').count();
    if isaret_sayisi > 0 {
        return SatirTuru::Alinti {
            derinlik: isaret_sayisi as u8,
            icerik: kirp[isaret_sayisi..].trim_start().to_string(),
        };
    }

    // Başlık: en fazla 3 boşluk girinti, 1..=6 `#`, sonra boşluk veya satır sonu.
    if girinti <= 3 {
        let hash = kirp.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hash) {
            let kalan = &kirp[hash..];
            if kalan.is_empty() || kalan.starts_with(' ') {
                return SatirTuru::Baslik {
                    seviye: hash as u8,
                    metin: kalan.trim().trim_end_matches('#').trim().to_string(),
                };
            }
        }
    }

    // İşaretçili liste: `-` / `*` / `+` ve ardından boşluk.
    if let Some(isaret) = kirp.chars().next() {
        if matches!(isaret, '-' | '*' | '+') && kirp.len() > 1 && kirp[1..].starts_with(' ') {
            return SatirTuru::Liste {
                girinti,
                metin: kirp[1..].trim().to_string(),
            };
        }
    }
    // Numaralı liste: `12.` veya `12)` ve ardından boşluk.
    let rakam = kirp.chars().take_while(char::is_ascii_digit).count();
    if rakam > 0 && rakam + 1 < kirp.len() {
        let ayirac = kirp.as_bytes()[rakam];
        if (ayirac == b'.' || ayirac == b')') && kirp[rakam + 1..].starts_with(' ') {
            return SatirTuru::Liste {
                girinti,
                metin: kirp[rakam + 1..].trim().to_string(),
            };
        }
    }

    SatirTuru::Duz {
        girinti,
        metin: kirp.to_string(),
    }
}

/// Liste maddelerinin başlıklardan sonra başlamasını sağlayan seviye tabanı.
const LISTE_SEVIYE_TABANI: u8 = 100;

/// Bir bloğun ağaç seviyesi.
fn konteyner_seviyesi(tip: BlokTipi, basamak: usize) -> u8 {
    match tip {
        BlokTipi::Baslik => (basamak as u8).clamp(1, 6),
        _ => LISTE_SEVIYE_TABANI.saturating_add((basamak / 2) as u8),
    }
}

/// Ayrıştırıcının geçici (arena tabanlı) düğüm temsili.
#[derive(Debug)]
struct Gecici {
    tip: BlokTipi,
    seviye: u8,
    yol: String,
    metin: String,
    ic_indent: usize,
    cocuklar: Vec<usize>,
    baslangic_satir: usize,
}

/// Bir alıntı satırından **tek** düzey `>` işaretçisini ve izleyen tek boşluğu
/// kaldırır. Kalan `>` işaretçileri alt ayrıştırmada iç içe alıntı üretir.
fn bir_duzey_soy(satir: &str) -> String {
    let k = satir.trim_start();
    match k.strip_prefix('>') {
        Some(kalan) => kalan.strip_prefix(' ').unwrap_or(kalan).to_string(),
        None => k.to_string(),
    }
}

/// Blok ağacını kademeli olarak kuran ayrıştırıcı.
struct Ayristirici {
    satirlar: Vec<String>,
    sira: usize,
    not_id: String,
    dugumler: Vec<Gecici>,
    koklar: Vec<usize>,
    acik: Vec<usize>,
    kod_isaret: Option<u8>,
    kod_satirlar: Vec<String>,
    kod_baslangic: usize,
    /// Son eklenen paragraf: `(düğüm indeksi, son satır, ebeveyn)`.
    ///
    /// Ardışık düz satırların **tek** paragraf oluşturmasını sağlar; metin
    /// birleştirildiği için blok kimliği de birleşik metinden hesaplanır.
    son_paragraf: Option<(usize, usize, Option<usize>)>,
}

impl Ayristirici {
    fn yeni(satirlar: &[&str], not_id: &str) -> Self {
        Self {
            satirlar: satirlar
                .iter()
                .map(|s| s.trim_end_matches('\r').to_string())
                .collect(),
            sira: 0,
            not_id: not_id.to_string(),
            dugumler: Vec::new(),
            koklar: Vec::new(),
            acik: Vec::new(),
            kod_isaret: None,
            kod_satirlar: Vec::new(),
            kod_baslangic: 0,
            son_paragraf: None,
        }
    }

    /// Yeni düğüm ekler ve indeksini döndürür.
    fn ekle(
        &mut self,
        tip: BlokTipi,
        metin: String,
        baslangic_satir: usize,
        ebeveyn: Option<usize>,
    ) -> usize {
        let sira = match ebeveyn {
            Some(p) => self.dugumler[p].cocuklar.len(),
            None => self.koklar.len(),
        };
        let yol = match ebeveyn {
            Some(p) => format!("{}.{sira}", self.dugumler[p].yol),
            None => sira.to_string(),
        };
        self.dugumler.push(Gecici {
            tip,
            seviye: 0,
            yol,
            metin,
            ic_indent: 0,
            cocuklar: Vec::new(),
            baslangic_satir,
        });
        let indeks = self.dugumler.len() - 1;
        match ebeveyn {
            Some(p) => self.dugumler[p].cocuklar.push(indeks),
            None => self.koklar.push(indeks),
        }
        indeks
    }

    /// Yaprak (paragraf/kod/alıntı) bloğunun ebeveynini girinti kuralına göre seçer.
    ///
    /// Açık yığın **en dıştan içe** taranır: girinti, bir konteynerin kabul
    /// eşiğini karşılıyorsa o konteyner seçilir. Eşiği karşılamayan konteynerler
    /// atlanır; hiçbiri karşılamazsa blok not köküne gider.
    fn ebeveyn_sec(&self, girinti: usize) -> Option<usize> {
        self.acik
            .iter()
            .rev()
            .find(|ust| girinti >= self.dugumler[**ust].ic_indent)
            .copied()
    }

    /// Konteyner (başlık veya liste maddesi) ekler: yığını derinliğe göre ayarlar,
    /// doğru ebeveyni bulur ve bloğu açar.
    fn konteyner_ekle(
        &mut self,
        tip: BlokTipi,
        metin: String,
        basamak: usize,
        girinti: usize,
        baslangic_satir: usize,
    ) {
        let seviye = konteyner_seviyesi(tip, basamak);
        while let Some(&ust) = self.acik.last() {
            if self.dugumler[ust].seviye >= seviye {
                self.acik.pop();
            } else {
                break;
            }
        }
        let ebeveyn = self.acik.last().copied();
        let indeks = self.ekle(tip, metin, baslangic_satir, ebeveyn);
        self.dugumler[indeks].seviye = seviye;
        // Başlıklar kendi düzeyindeki her şeyi kabul eder; bir madde ancak daha
        // derin girintili devam satırlarını kabul eder.
        self.dugumler[indeks].ic_indent = if tip == BlokTipi::Baslik {
            0
        } else {
            girinti + 2
        };
        self.acik.push(indeks);
    }

    /// Alıntı bloğunu kurar: yalnızca **bir** düzey `>` işaretçisi soyulur,
    /// kalan metin yeniden ayrıştırılır.
    ///
    /// Böylece `>> metin` iç içe iki alıntı üretir ve `> - madde` doğru biçimde
    /// bir liste madde(si) üretir.
    fn alinti_kur(&mut self, satirlar: Vec<String>, baslangic: usize) {
        let ebeveyn = self.ebeveyn_sec(0);
        let indeks = self.ekle(BlokTipi::Alinti, String::new(), baslangic, ebeveyn);
        let alt = satirlar.join("\n");
        let alt_id = format!("{}#alinti", self.not_id);
        for blok in ayristir(&alt, &alt_id) {
            self.alt_ekle(indeks, &blok, baslangic);
        }
    }

    /// Yeniden ayrıştırılmış alt ağacı arena'ya yol önekiyle birlikte gömer.
    fn alt_ekle(&mut self, ebeveyn: usize, blok: &Blok, satir_ofseti: usize) {
        let yol = format!("{}.{}", self.dugumler[ebeveyn].yol, blok.yol);
        self.dugumler.push(Gecici {
            tip: blok.tip,
            seviye: 0,
            yol,
            metin: blok.metin.clone(),
            ic_indent: 0,
            cocuklar: Vec::new(),
            baslangic_satir: satir_ofseti.saturating_add(blok.baslangic_satir - 1),
        });
        let indeks = self.dugumler.len() - 1;
        self.dugumler[ebeveyn].cocuklar.push(indeks);
        for cocuk in &blok.cocuklar {
            self.alt_ekle(indeks, cocuk, satir_ofseti);
        }
    }

    fn kod_kapat(&mut self) {
        let metin = std::mem::take(&mut self.kod_satirlar).join("\n");
        let baslangic = self.kod_baslangic;
        self.kod_isaret = None;
        let ebeveyn = self.ebeveyn_sec(0);
        self.ekle(BlokTipi::Kod, metin, baslangic, ebeveyn);
    }

    fn calistir(&mut self) {
        let toplam = self.satirlar.len();
        while self.sira < toplam {
            let ham = self.satirlar[self.sira].clone();
            let turu = parcala_satir(&ham);

            // Kod bloğu içindeysek yalnızca kapanışı ararız.
            if let Some(isaret) = self.kod_isaret {
                match turu {
                    SatirTuru::Kod { isaret: k, .. } if k == isaret => self.kod_kapat(),
                    _ => self.kod_satirlar.push(ham),
                }
                self.sira += 1;
                continue;
            }
            // Paragraf birleştirme durumu yalnızca düz satırlarda korunur; diğer
            // her blok türü zinciri kırar.
            let onceki_paragraf = self.son_paragraf.take();

            match turu {
                SatirTuru::Bos => self.sira += 1,
                SatirTuru::Kod { isaret, .. } => {
                    self.kod_isaret = Some(isaret);
                    self.kod_baslangic = self.sira + 1;
                    self.kod_satirlar.clear();
                    self.sira += 1;
                }
                SatirTuru::Baslik { seviye, metin } => {
                    self.konteyner_ekle(BlokTipi::Baslik, metin, seviye as usize, 0, self.sira + 1);
                    self.sira += 1;
                }
                SatirTuru::Liste { girinti, metin } => {
                    self.konteyner_ekle(BlokTipi::Liste, metin, girinti, girinti, self.sira + 1);
                    self.sira += 1;
                }
                SatirTuru::Alinti { derinlik, .. } => {
                    let baslangic = self.sira + 1;
                    let mut toplanan = vec![bir_duzey_soy(&ham)];
                    self.sira += 1;
                    while self.sira < toplam {
                        match parcala_satir(&self.satirlar[self.sira]) {
                            SatirTuru::Alinti { derinlik: d, .. } if d >= derinlik => {
                                toplanan.push(bir_duzey_soy(&self.satirlar[self.sira]));
                                self.sira += 1;
                            }
                            _ => break,
                        }
                    }
                    self.alinti_kur(toplanan, baslangic);
                }
                SatirTuru::Duz { girinti, metin } => {
                    let ebeveyn = self.ebeveyn_sec(girinti);
                    let satir = self.sira + 1;
                    // Önceki satır da aynı ebeveynde bir paragraf ise ekle.
                    let devam = matches!(
                        onceki_paragraf,
                        Some((_, son_satir, ust)) if son_satir + 1 == satir && ust == ebeveyn
                    );
                    if let (true, Some((indeks, _, _))) = (devam, onceki_paragraf) {
                        self.dugumler[indeks].metin.push('\n');
                        self.dugumler[indeks].metin.push_str(&metin);
                        self.son_paragraf = Some((indeks, satir, ebeveyn));
                    } else {
                        let indeks = self.ekle(BlokTipi::Paragraf, metin, satir, ebeveyn);
                        self.son_paragraf = Some((indeks, satir, ebeveyn));
                    }
                    self.sira += 1;
                }
            }
        }

        // Kapanmamış kod bloğu: kalan satırlar yine de bir kod bloğudur.
        if self.kod_isaret.is_some() {
            self.kod_kapat();
        }
    }

    /// Arena ağacını iç içe [`Blok`] ağacına çevirir.
    ///
    /// Kimlik, tam burada ve **yalnızca burada** hesaplanır: yol ön eklenmiş hâliyle
    /// kullanılır, böylece alıntı içi bloklar da notun tam yolundan türer.
    /// Aynı kimliği taşıyan ikinci bir düğüm varsa `-2`, `-3` eklenerek
    /// çakışma kararlı biçimde çözülür.
    fn koklari_cikar(&mut self) -> Vec<Blok> {
        let koklar = self.koklar.clone();
        let mut sayaclar: Vec<(String, usize)> = Vec::new();
        let mut agac = Vec::with_capacity(koklar.len());
        for indeks in koklar {
            if let Some(blok) = self.agaca_cevir(indeks, 0, &mut sayaclar) {
                agac.push(blok);
            }
        }
        agac
    }

    fn agaca_cevir(
        &self,
        indeks: usize,
        derinlik: u8,
        sayaclar: &mut Vec<(String, usize)>,
    ) -> Option<Blok> {
        let g = &self.dugumler[indeks];
        let temel = blok_kimligi(&self.not_id, &g.yol, g.tip.etiket(), &g.metin);
        let adet = match sayaclar.iter_mut().find(|(k, _)| *k == temel) {
            Some((_, n)) => {
                *n += 1;
                *n
            }
            None => {
                sayaclar.push((temel.clone(), 1));
                1
            }
        };
        let kimlik = if adet > 1 {
            format!("{temel}-{}", adet - 1)
        } else {
            temel
        };

        let mut cocuklar = Vec::with_capacity(g.cocuklar.len());
        for &c in &g.cocuklar {
            if let Some(b) = self.agaca_cevir(c, derinlik.saturating_add(1), sayaclar) {
                cocuklar.push(b);
            }
        }

        Some(Blok {
            kimlik,
            tip: g.tip,
            yol: g.yol.clone(),
            metin: g.metin.clone(),
            derinlik,
            baslangic_satir: g.baslangic_satir,
            cocuklar,
        })
    }
}

/// Bir Markdown metnini blok ağacına ayrıştırır ve her bloğa kararlı kimlik verir.
///
/// `not_id`, blok kimliklerinin türetilmesinde kullanılır; aynı `not_id` ve aynı
/// metin her zaman aynı ağacı (ve aynı kimlikleri) üretir.
pub fn ayristir(metin: &str, not_id: &str) -> Vec<Blok> {
    // Windows düzenleyicileri dosya başına UTF-8 BOM (U+FEFF) yazabiliyor.
    // BOM, ilk satırı bozan görünmez bir karakterdir; başlık tanımasını bozmasın
    // diye metnin başından atılır.
    let metin = metin.strip_prefix('\u{feff}').unwrap_or(metin);
    let satirlar: Vec<&str> = metin.split('\n').collect();
    let mut ay = Ayristirici::yeni(&satirlar, not_id);
    ay.calistir();
    ay.koklari_cikar()
}

/// Bir blok ağacını önce kökler olmak üzere derinlik sırasıyla düzleştirir.
pub fn duzlestir(bloklar: &[Blok]) -> Vec<&Blok> {
    let mut cikti = Vec::new();
    for b in bloklar {
        duzlestir_ic(b, &mut cikti);
    }
    cikti
}

fn duzlestir_ic<'a>(blok: &'a Blok, cikti: &mut Vec<&'a Blok>) {
    cikti.push(blok);
    for c in &blok.cocuklar {
        duzlestir_ic(c, cikti);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn turler(bloklar: &[Blok]) -> Vec<BlokTipi> {
        duzlestir(bloklar).into_iter().map(|b| b.tip).collect()
    }

    fn metinler(bloklar: &[Blok]) -> Vec<String> {
        duzlestir(bloklar)
            .into_iter()
            .map(|b| b.metin.clone())
            .collect()
    }

    // ---- parcala_satir ----

    #[test]
    fn bos_satir_bos_donulur() {
        assert_eq!(parcala_satir(""), SatirTuru::Bos);
        assert_eq!(parcala_satir("   \t "), SatirTuru::Bos);
    }

    #[test]
    fn baslik_sinifi_ayrilir() {
        assert_eq!(
            parcala_satir("# Baslik"),
            SatirTuru::Baslik {
                seviye: 1,
                metin: "Baslik".to_string()
            }
        );
        assert_eq!(
            parcala_satir("###### Alt"),
            SatirTuru::Baslik {
                seviye: 6,
                metin: "Alt".to_string()
            }
        );
    }

    #[test]
    fn baslik_kapanis_hashes_temizlenir() {
        assert_eq!(
            parcala_satir("## Baslik ##"),
            SatirTuru::Baslik {
                seviye: 2,
                metin: "Baslik".to_string()
            }
        );
    }

    #[test]
    fn yedi_hash_baslik_degil_duz_metindir() {
        assert!(matches!(
            parcala_satir("####### yedi"),
            SatirTuru::Duz { .. }
        ));
        assert!(matches!(parcala_satir("#YokBosluk"), SatirTuru::Duz { .. }));
    }

    #[test]
    fn girintili_hash_duz_metindir() {
        assert!(matches!(
            parcala_satir("    # girintili"),
            SatirTuru::Duz { .. }
        ));
    }

    #[test]
    fn liste_isaretleri_taninir() {
        for isaret in ["- ", "* ", "+ "] {
            let satir = format!("{isaret}madde");
            assert_eq!(
                parcala_satir(&satir),
                SatirTuru::Liste {
                    girinti: 0,
                    metin: "madde".to_string()
                },
                "isaret {isaret} taninmali"
            );
        }
    }

    #[test]
    fn numarali_liste_taninir() {
        assert_eq!(
            parcala_satir("12. madde"),
            SatirTuru::Liste {
                girinti: 0,
                metin: "madde".to_string()
            }
        );
        assert_eq!(
            parcala_satir("3) madde"),
            SatirTuru::Liste {
                girinti: 0,
                metin: "madde".to_string()
            }
        );
    }

    #[test]
    fn liste_girintisi_olculur() {
        assert_eq!(
            parcala_satir("    - ic madde"),
            SatirTuru::Liste {
                girinti: 4,
                metin: "ic madde".to_string()
            }
        );
    }

    #[test]
    fn tire_tek_basina_duz_metindir() {
        assert!(matches!(parcala_satir("-"), SatirTuru::Duz { .. }));
        assert!(matches!(
            parcala_satir("-bosluk yok"),
            SatirTuru::Duz { .. }
        ));
    }

    #[test]
    fn alinti_derinligi_sayilir() {
        assert_eq!(
            parcala_satir("> alinti"),
            SatirTuru::Alinti {
                derinlik: 1,
                icerik: "alinti".to_string()
            }
        );
        assert_eq!(
            parcala_satir(">> ic"),
            SatirTuru::Alinti {
                derinlik: 2,
                icerik: "ic".to_string()
            }
        );
    }

    #[test]
    fn kod_cesiti_taninir() {
        assert_eq!(
            parcala_satir("```rust"),
            SatirTuru::Kod {
                isaret: 3,
                dil: "rust".to_string()
            }
        );
        assert_eq!(
            parcala_satir("```"),
            SatirTuru::Kod {
                isaret: 3,
                dil: String::new()
            }
        );
        assert_eq!(
            parcala_satir("~~~~"),
            SatirTuru::Kod {
                isaret: 4,
                dil: String::new()
            }
        );
        // İki tilde bir kod çiti değildir.
        assert!(matches!(parcala_satir("~~not~~"), SatirTuru::Duz { .. }));
    }

    // ---- ayristir ----

    #[test]
    fn bos_not_hicbir_blok_uretmez() {
        assert!(ayristir("", "n.md").is_empty());
    }

    #[test]
    fn yalniz_bosluklu_not_hicbir_blok_uretmez() {
        assert!(ayristir("\n\n   \n\t\n", "n.md").is_empty());
    }

    #[test]
    fn tek_paragraf_kok_blok_olur() {
        let agac = ayristir("Tek paragraf.", "n.md");
        assert_eq!(agac.len(), 1);
        assert_eq!(agac[0].tip, BlokTipi::Paragraf);
        assert_eq!(agac[0].derinlik, 0);
        assert_eq!(agac[0].baslangic_satir, 1);
    }

    #[test]
    fn ardisik_duz_satirlar_tek_paragrapha_birlenir() {
        let agac = ayristir("bir iki\nuc dort", "n.md");
        assert_eq!(agac.len(), 1);
        assert_eq!(agac[0].metin, "bir iki\nuc dort");
    }

    #[test]
    fn baslik_higherasi_agac_olusur() {
        let agac = ayristir("# A\n## B\n### C\n## D", "n.md");
        assert_eq!(agac.len(), 1);
        assert_eq!(agac[0].cocuklar.len(), 2);
        assert_eq!(agac[0].cocuklar[0].metin, "B");
        assert_eq!(agac[0].cocuklar[0].cocuklar[0].metin, "C");
        assert_eq!(agac[0].cocuklar[0].cocuklar[0].derinlik, 2);
        assert_eq!(agac[0].cocuklar[1].metin, "D");
    }

    #[test]
    fn baslik_altindaki_paragraf_cocuktur() {
        let agac = ayristir("# A\n\nGiris paragrafi.", "n.md");
        assert_eq!(agac[0].cocuklar.len(), 1);
        assert_eq!(agac[0].cocuklar[0].tip, BlokTipi::Paragraf);
    }

    #[test]
    fn ic_ice_liste_maddeleri_agac_olusur() {
        let agac = ayristir("- a\n  - b\n    - c\n- d", "n.md");
        assert_eq!(agac.len(), 2);
        assert_eq!(agac[0].metin, "a");
        assert_eq!(agac[0].cocuklar[0].metin, "b");
        assert_eq!(agac[0].cocuklar[0].cocuklar[0].metin, "c");
        assert_eq!(agac[1].metin, "d");
    }

    #[test]
    fn liste_devam_satiri_maddenin_cocugu_olur() {
        let agac = ayristir("- a\n    devam satiri", "n.md");
        assert_eq!(agac[0].cocuklar.len(), 1);
        assert_eq!(agac[0].cocuklar[0].tip, BlokTipi::Paragraf);
        assert_eq!(agac[0].cocuklar[0].metin, "devam satiri");
    }

    #[test]
    fn girintisiz_devam_satiri_kardes_kalir() {
        let agac = ayristir("- a\nkardes metni", "n.md");
        assert_eq!(agac.len(), 2);
        assert_eq!(agac[1].tip, BlokTipi::Paragraf);
    }

    #[test]
    fn baslik_ten_cok_listesi_maddenin_cocugu_olur() {
        let agac = ayristir("# A\n- a\n  - b", "n.md");
        assert_eq!(agac[0].cocuklar.len(), 1);
        assert_eq!(agac[0].cocuklar[0].metin, "a");
        assert_eq!(agac[0].cocuklar[0].cocuklar[0].metin, "b");
    }

    #[test]
    fn alinti_islaretci_soyulur_ve_yeniden_ayristirilir() {
        let agac = ayristir("> # Baslik\n> - madde", "n.md");
        assert_eq!(agac.len(), 1);
        assert_eq!(agac[0].tip, BlokTipi::Alinti);
        // Alıntının alt metni yeniden ayrıştırılır; bir liste maddesi her zaman
        // başlıktan derin olduğu için madde, başlığın çocuğudur.
        assert_eq!(
            turler(&agac),
            vec![BlokTipi::Alinti, BlokTipi::Baslik, BlokTipi::Liste]
        );
        assert_eq!(agac[0].cocuklar[0].metin, "Baslik");
        assert_eq!(agac[0].cocuklar[0].cocuklar[0].metin, "madde");
    }

    #[test]
    fn alinti_iki_kardes_uretir() {
        let agac = ayristir("> - bir\n> - iki", "n.md");
        assert_eq!(agac.len(), 1);
        assert_eq!(agac[0].cocuklar.len(), 2);
        assert_eq!(agac[0].cocuklar[0].metin, "bir");
        assert_eq!(agac[0].cocuklar[1].metin, "iki");
    }

    #[test]
    fn alinti_ardisik_duz_satirlari_birlestirir() {
        // Alıntı alt metni de normal kurallarla ayrıştırılır: ardışık düz
        // satırlar tek paragraf olur.
        let agac = ayristir("> birinci\n> ikinci", "n.md");
        assert_eq!(agac[0].cocuklar.len(), 1);
        assert_eq!(agac[0].cocuklar[0].metin, "birinci\nikinci");
    }

    #[test]
    fn ic_alinti_ic_alinti_uretir() {
        let agac = ayristir(">> ic", "n.md");
        assert_eq!(agac[0].tip, BlokTipi::Alinti);
        assert_eq!(agac[0].cocuklar[0].tip, BlokTipi::Alinti);
    }

    #[test]
    fn alinti_ardindan_duz_satir_kardes_kalir() {
        let agac = ayristir("> alinti\nkardes", "n.md");
        assert_eq!(agac.len(), 2);
        assert_eq!(agac[1].tip, BlokTipi::Paragraf);
    }

    #[test]
    fn alinti_sonrasi_madde_yeniden_kok_acilir() {
        let agac = ayristir("# A\n> alinti\n- madde", "n.md");
        assert_eq!(agac[0].cocuklar.len(), 2);
        assert_eq!(agac[0].cocuklar[0].tip, BlokTipi::Alinti);
        assert_eq!(agac[0].cocuklar[1].tip, BlokTipi::Liste);
    }

    #[test]
    fn kod_bloku_tek_blok_olur_ve_icerigi_ayristirilmaz() {
        let agac = ayristir("```\n# bu baslik degil\n- bu madde degil\n```", "n.md");
        assert_eq!(agac.len(), 1);
        assert_eq!(agac[0].tip, BlokTipi::Kod);
        assert_eq!(agac[0].metin, "# bu baslik degil\n- bu madde degil");
    }

    #[test]
    fn kapanmamis_kod_bloku_da_kaybolmaz() {
        let agac = ayristir("```\nson satirsiz", "n.md");
        assert_eq!(agac.len(), 1);
        assert_eq!(agac[0].tip, BlokTipi::Kod);
        assert_eq!(agac[0].metin, "son satirsiz");
    }

    #[test]
    fn uzun_cit_icinde_kisa_cit_kapanis_degildir() {
        let agac = ayristir("````\n```\n````", "n.md");
        assert_eq!(agac[0].tip, BlokTipi::Kod);
        assert_eq!(agac[0].metin, "```");
    }

    #[test]
    fn baglanti_isaretci_metinde_korunur() {
        let agac = ayristir("Sirasiyla [[bir]] ve ((abc)) bagli.", "n.md");
        assert_eq!(agac[0].metin, "Sirasiyla [[bir]] ve ((abc)) bagli.");
    }

    #[test]
    fn blok_kimligi_aynistirmada_ayni_kalir() {
        let metin = "# A\n- b\n  - c\n\nparagraf";
        let a = ayristir(metin, "n.md");
        let b = ayristir(metin, "n.md");
        let kimlikler = |agac: &[Blok]| -> Vec<String> {
            duzlestir(agac)
                .into_iter()
                .map(|x| x.kimlik.clone())
                .collect()
        };
        assert_eq!(kimlikler(&a), kimlikler(&b));
        assert_eq!(kimlikler(&a).len(), 4);
    }

    #[test]
    fn blok_kimligi_not_id_ile_degisir() {
        assert_ne!(
            ayristir("# A", "bir.md")[0].kimlik,
            ayristir("# A", "iki.md")[0].kimlik
        );
    }

    #[test]
    fn ayni_metni_iki_kez_icen_bloklar_farkli_kimlik_alir() {
        let agac = ayristir("# Tek\n# Tek", "n.md");
        assert_eq!(agac.len(), 2);
        assert_ne!(agac[0].kimlik, agac[1].kimlik);
    }

    #[test]
    fn yollar_ve_derinlikler_tutarli() {
        let agac = ayristir("# A\n## B\n### C", "n.md");
        let duz = duzlestir(&agac);
        assert_eq!(duz[0].yol, "0");
        assert_eq!(duz[1].yol, "0.0");
        assert_eq!(duz[2].yol, "0.0.0");
        assert_eq!(duz[2].derinlik, 2);
    }

    #[test]
    fn alinti_cocuklarinin_kimligi_not_yolunden_turulur() {
        let tek = ayristir("> paragraf", "n.md");
        let iki = ayristir("# A\n> paragraf", "n.md");
        assert_ne!(
            tek[0].cocuklar[0].kimlik,
            iki[0].cocuklar[0].cocuklar[0].kimlik
        );
    }

    #[test]
    fn blok_tipi_etiket_ve_agirliklari_ayrilir() {
        assert_eq!(BlokTipi::Baslik.etiket(), "baslik");
        assert_eq!(BlokTipi::Liste.etiket(), "madde");
        assert!(BlokTipi::Baslik.alan_agirligi() > BlokTipi::Paragraf.alan_agirligi());
        assert!(BlokTipi::Paragraf.alan_agirligi() > BlokTipi::Kod.alan_agirligi());
    }

    #[test]
    fn karisik_dokuman_ayristirilir() {
        let metin =
            "# Baslik\n\nGiris paragrafi.\n\n- bir\n  - iki\n\n> alinti\n\n```\nkod\n```\n\nSon.";
        // Alıntı bloğu kendi paragraf çocuğunu taşır: `> alinti` bir "alıntı ->
        // paragraf" ağacıdır, düz bir paragraf değil.
        assert_eq!(
            turler(&ayristir(metin, "n.md")),
            vec![
                BlokTipi::Baslik,
                BlokTipi::Paragraf,
                BlokTipi::Liste,
                BlokTipi::Liste,
                BlokTipi::Alinti,
                BlokTipi::Paragraf,
                BlokTipi::Kod,
                BlokTipi::Paragraf
            ]
        );
    }

    #[test]
    fn satir_sonu_karakteri_temizlenir() {
        let agac = ayristir("# A\r\n- b\r\n", "n.md");
        assert_eq!(metinler(&agac), vec!["A", "b"]);
    }

    #[test]
    fn bom_ilk_basligi_bozmaz() {
        // Windows düzenleyicilerinin eklediği UTF-8 BOM görünmez bir karakterdir.
        let agac = ayristir("\u{feff}# A\n\n- b\n", "n.md");
        assert_eq!(turler(&agac), vec![BlokTipi::Baslik, BlokTipi::Liste]);
        assert_eq!(agac[0].metin, "A");
    }

    #[test]
    fn buyuk_dokuman_yuzlerce_blok_uretir() {
        let mut metin = String::new();
        for i in 0..200 {
            metin.push_str(&format!("# Baslik {i}\n- madde {i}\n\nparagraf {i}\n\n"));
        }
        let agac = ayristir(&metin, "n.md");
        assert_eq!(agac.len(), 200);
        assert_eq!(duzlestir(&agac).len(), 600);
        let benzersiz: std::collections::HashSet<String> = duzlestir(&agac)
            .into_iter()
            .map(|b| b.kimlik.clone())
            .collect();
        assert_eq!(benzersiz.len(), 600);
    }
}
