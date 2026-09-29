//! İndeks dosyasının şeması, sürüm denetimi ve JSON gidiş-dönüşü.
//!
//! İndeks **türetilmiş veridir** (rapor b07, "Veri modeli"): notlardan üretilir,
//! silinebilir ve yeniden üretilebilir. Bu yüzden biçim sürümlenir; sürüm
//! uyuşmazlığında dosya sessizce kullanılmaz, açık bir hata döner ve kullanıcıya
//! `nodemind index` ile yeniden kurması söylenir.
//!
//! Dosya yolu: `<kasa>/cache/index.json` (rapor b09, şema 4).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::hata::Hata;
use crate::markdown::BlokTipi;

/// İndeks dosyasının şema sürümü.
///
/// Bu sayı, aşağıdaki alanların **anlamı** değiştiğinde artırılmalıdır. Yeni
/// alan eklemek (geriye uyumlu) artırma gerektirmez.
pub const SEMA_SURUMU: u32 = 1;

/// Bu ikilinin sürümü (Cargo.toml'dan okunur).
pub const SURUM: &str = env!("CARGO_PKG_VERSION");

/// Arama sonucu önizlemesi için saklanan azami karakter sayısı (blok kaydının `ozet` alanı).
pub const OZET_ADEDI: usize = 160;

/// `cache` alt dizininin adı (rapor b09: "silinebilir: yeniden kurulabilir").
pub const CACHE_DIZINI: &str = "cache";

/// İndeks dosyasının adı.
pub const INDEKS_DOSYASI: &str = "index.json";

/// Bir bloğun indeks dosyasındaki kaydı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlokKaydi {
    /// Blok kimliği (arama sonucunu bloğa bağlayan anahtar).
    pub id: String,
    /// Bloğun ait olduğu notun kimliği.
    pub not_id: String,
    /// Bloğun ait olduğu notun okunabilir adı.
    pub baslik: String,
    /// Bloğun ait olduğu notun kasa köküne göreli yolu (çıktıda okunabilirlik için).
    pub not_yol: String,
    /// Bloğun başlıktan başlayarak okunabilir yolu (`"Giris > Ayrinti"`).
    pub baslik_yolu: String,
    /// Blok türü (`"snake_case"` adıyla serileştirilir).
    pub tip: BlokTipi,
    /// Kısaltılmış metin önizlemesi.
    pub ozet: String,
    /// 1 tabanlı satır numarası.
    pub satir: usize,
}

/// Bir notun indeks dosyasındaki kaydı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotKaydi {
    /// Notun göreli yolundan türetilmiş kararlı kimliği.
    pub id: String,
    /// Kasanın köküne göreli yol (`/` ayraçlı, ör. `"gunluk/2026-09-29.md"`).
    pub yol: String,
    /// Dosya adı (uzantısız).
    pub ad: String,
    /// Notun görünen başlığı (ilk `#` başlık, yoksa dosya adı).
    pub baslik: String,
    /// Dosya adından çözülen günlük tarihi (`YYYY-AA-GG`), günlük değilse `null`.
    pub tarih: Option<String>,
    /// Dosyanın son değişiklik zamanı (Unix saniye). Önbellek tazeliğini belirler.
    pub degisiklik: u64,
}

/// İndeks dosyasının tamamı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndeksBelge {
    /// Şema sürümü; [`SEMA_SURUMU`] ile eşleşmelidir.
    pub sema: u32,
    /// İndeksi üreten ikilinin sürümü (bilgi amaçlı, eşleşme zorunlu değil).
    pub surum: String,
    /// İndeksin oluşturulduğu Unix saniyesi.
    pub olusturma: u64,
    /// Taranan not sayısı.
    pub not_sayisi: usize,
    /// İnlenen blok sayısı.
    pub blok_sayisi: usize,
    /// Not kayıtları (kimliğe göre sıralı).
    pub notlar: Vec<NotKaydi>,
    /// Blok kayıtları (kimliğe göre sıralı).
    pub bloklar: Vec<BlokKaydi>,
    /// Ters indeks çıktısı: `terim -> (blok kimliği -> frekans)`.
    pub terimler: BTreeMap<String, BTreeMap<String, u32>>,
}

impl IndeksBelge {
    /// Boş, geçerli sürümlü bir indeks belgesi oluşturur.
    pub fn yeni(olusturma: u64) -> Self {
        Self {
            sema: SEMA_SURUMU,
            surum: SURUM.to_string(),
            olusturma,
            not_sayisi: 0,
            blok_sayisi: 0,
            notlar: Vec::new(),
            bloklar: Vec::new(),
            terimler: BTreeMap::new(),
        }
    }

    /// Belgenin şema sürümü bu ikiliyle uyumlu mu?
    pub fn uyumlu_mu(&self) -> bool {
        self.sema == SEMA_SURUMU
    }

    /// Blok kayıtlarını kimliğe göre sıralanmış bir haritaya çevirir.
    pub fn blok_haritasi(&self) -> BTreeMap<String, BlokKaydi> {
        self.bloklar
            .iter()
            .map(|b| (b.id.clone(), b.clone()))
            .collect()
    }

    /// Not adı → not kaydı haritası (yenilen adlarda ilki kazanır).
    pub fn not_haritasi(&self) -> BTreeMap<String, NotKaydi> {
        let mut m = BTreeMap::new();
        for n in &self.notlar {
            m.entry(n.ad.clone()).or_insert_with(|| n.clone());
        }
        m
    }

    /// Belgeyi okunabilir JSON metnine çevirir (sonlu satır sonu, girintili).
    pub fn metin(&self) -> Result<String, Hata> {
        serde_json::to_string_pretty(self).map_err(|e| Hata::CiktiHatasi {
            yol: std::path::PathBuf::from("<bellek>"),
            kaynak: std::io::Error::other(e),
        })
    }

    /// JSON metninden belgeyi geri okur.
    pub fn ayristir(metin: &str) -> Result<Self, Hata> {
        serde_json::from_str(metin).map_err(|e| Hata::BozukIndeks {
            yol: std::path::PathBuf::from("<bellek>"),
            sebep: e.to_string(),
        })
    }

    /// İndeks dosyasını diskten okur; **şema uyuşmazlığında hata döner**.
    pub fn yukle(yol: &Path) -> Result<Self, Hata> {
        let metin = std::fs::read_to_string(yol)
            .map_err(|e| crate::hata::io_hata("indeks okuma", yol, e))?;
        let belge: Self = serde_json::from_str(&metin).map_err(|e| Hata::BozukIndeks {
            yol: yol.to_path_buf(),
            sebep: e.to_string(),
        })?;
        if !belge.uyumlu_mu() {
            return Err(Hata::SemaUyusmazligi {
                bulunan: belge.sema,
                beklenen: SEMA_SURUMU,
                yol: yol.to_path_buf(),
            });
        }
        Ok(belge)
    }

    /// İndeks dosyasını diske yazar; üst dizini yoksa oluşturur.
    pub fn kaydet(&self, yol: &Path) -> Result<(), Hata> {
        if let Some(ust) = yol.parent() {
            std::fs::create_dir_all(ust)
                .map_err(|e| crate::hata::io_hata("cache dizini olusturma", ust, e))?;
        }
        let metin = serde_json::to_string_pretty(self).map_err(|e| Hata::CiktiHatasi {
            yol: yol.to_path_buf(),
            kaynak: std::io::Error::other(e),
        })?;
        std::fs::write(yol, metin).map_err(|e| crate::hata::io_hata("indeks yazma", yol, e))
    }
}

/// Bir metinden arama önizlemesi üretir.
pub fn ozet_uret(metin: &str, adet: usize) -> String {
    crate::indeks::ozetle(metin, adet)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::indeks::TersIndeks;

    fn dolu_belge() -> IndeksBelge {
        let mut b = IndeksBelge::yeni(1_700_000_000);
        b.notlar.push(NotKaydi {
            id: "n1".to_string(),
            yol: "gunluk/2026-09-29.md".to_string(),
            ad: "2026-09-29".to_string(),
            baslik: "Gunluk".to_string(),
            tarih: Some("2026-09-29".to_string()),
            degisiklik: 42,
        });
        b.notlar.push(NotKaydi {
            id: "n2".to_string(),
            yol: "markdown-lehcesi.md".to_string(),
            ad: "markdown-lehcesi".to_string(),
            baslik: "Markdown Lehcesi".to_string(),
            tarih: None,
            degisiklik: 43,
        });
        b.bloklar.push(BlokKaydi {
            id: "b1".to_string(),
            not_id: "n1".to_string(),
            baslik: "Gunluk".to_string(),
            not_yol: "gunluk/2026-09-29.md".to_string(),
            baslik_yolu: "Gunluk > Bugun".to_string(),
            tip: BlokTipi::Baslik,
            ozet: "Gunluk".to_string(),
            satir: 1,
        });
        b.not_sayisi = 2;
        b.blok_sayisi = 1;
        let mut t = TersIndeks::yeni();
        t.ekle("b1", "gunluk markdown");
        b.terimler = t.terimler_disa();
        b
    }

    #[test]
    fn yeni_belge_guncel_surumu_tasir() {
        let b = IndeksBelge::yeni(0);
        assert_eq!(b.sema, SEMA_SURUMU);
        assert_eq!(b.surum, SURUM);
        assert!(b.uyumlu_mu());
        assert_eq!(b.not_sayisi, 0);
    }

    #[test]
    fn json_gidis_donusu_kimliktir() {
        let a = dolu_belge();
        let metin = a.metin().unwrap();
        let b = IndeksBelge::ayristir(&metin).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn json_gidis_donusu_iki_kez_ayni_metini_uretilir() {
        let a = dolu_belge();
        let b = IndeksBelge::ayristir(&a.metin().unwrap()).unwrap();
        assert_eq!(a.metin().unwrap(), b.metin().unwrap());
    }

    #[test]
    fn blok_tipi_snake_case_olarak_serilestirilir() {
        let metin = dolu_belge().metin().unwrap();
        assert!(metin.contains("\"tip\": \"baslik\""), "{metin}");
    }

    #[test]
    fn tarih_alani_null_olarak_yazilir() {
        let metin = dolu_belge().metin().unwrap();
        assert!(metin.contains("\"tarih\": null"), "{metin}");
    }

    #[test]
    fn uyumsuz_sema_yuklemede_hata_dondurur() {
        let mut a = dolu_belge();
        a.sema = SEMA_SURUMU + 7;
        assert!(!a.uyumlu_mu());
        // Serileştirme/deserileştirme şemayı taşır ama **denetlemez**; denetim
        // `yukle` ve `nodemind::indeks_hazirla` katmanındadır.
        let b = IndeksBelge::ayristir(&a.metin().unwrap()).unwrap();
        assert!(!b.uyumlu_mu());
        assert!(crate::indeks_hazirla(&b).is_err());
    }

    #[test]
    fn bozuk_json_hata_dondurur() {
        let h = IndeksBelge::ayristir("{bozuk").unwrap_err();
        assert!(h.to_string().contains("okunamadi"), "{h}");
    }

    #[test]
    fn blok_haritasi_kimlige_gore_siralanir() {
        let b = dolu_belge();
        let m = b.blok_haritasi();
        assert_eq!(m.len(), 1);
        assert_eq!(m.get("b1").unwrap().not_id, "n1");
    }

    #[test]
    fn not_haritasi_ada_gore_siralanir() {
        let m = dolu_belge().not_haritasi();
        assert_eq!(m.len(), 2);
        assert!(m.contains_key("2026-09-29"));
        assert!(m.contains_key("markdown-lehcesi"));
    }

    #[test]
    fn terim_disa_aktarimi_ters_indeksle_ayni() {
        let mut t = TersIndeks::yeni();
        t.ekle("b1", "bir iki");
        let d = t.terimler_disa();
        assert_eq!(d.get("bir").unwrap().get("b1"), Some(&1));
    }

    #[test]
    fn ozet_uret_kisaltir() {
        let o = ozet_cret_yardimci();
        assert!(o.chars().count() <= 10);
    }

    fn ozet_cret_yardimci() -> String {
        ozet_uret("bir iki uc dort bes alti yedi", 10)
    }

    #[test]
    fn yukle_uyumsuz_sema_dosyasinda_hata_verir() {
        let d = crate::test_yardimcisi::GeciciDizin::yeni("nodemind-sema").unwrap();
        let yol = d.yol().join("index.json");
        let mut a = dolu_belge();
        a.sema = 99;
        std::fs::write(&yol, a.metin().unwrap()).unwrap();
        let h = IndeksBelge::yukle(&yol).unwrap_err();
        assert!(h.to_string().contains("semasi uyusmuyor"), "{h}");
    }

    #[test]
    fn kaydet_ve_yukle_gidis_donusu() {
        let d = crate::test_yardimcisi::GeciciDizin::yeni("nodemind-sema-io").unwrap();
        let yol = d.yol().join("cache").join("index.json");
        let a = dolu_belge();
        a.kaydet(&yol).unwrap();
        assert!(yol.exists());
        let b = IndeksBelge::yukle(&yol).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn yukle_olmayan_dosyada_io_hatasi_verir() {
        let d = crate::test_yardimcisi::GeciciDizin::yeni("nodemind-sema-ok").unwrap();
        let yol = d.yol().join("yok.json");
        assert!(IndeksBelge::yukle(&yol).is_err());
    }

    #[test]
    fn sabitler_beklenen_degerlerdedir() {
        assert_eq!(CACHE_DIZINI, "cache");
        assert_eq!(INDEKS_DOSYASI, "index.json");
        let ozet = std::hint::black_box(OZET_ADEDI);
        assert!((80..=400).contains(&ozet));
    }
}
