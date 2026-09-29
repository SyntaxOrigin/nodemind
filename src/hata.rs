//! Hata tipleri ve bunların `Display` uygulamaları.
//!
//! Sorumluluğu, tüm modüllerin ortak kullandığı tek hata sınıfını tanımlamaktır.
//! `thiserror` bağımlılığı yasak olduğu için (`WORKER_CONTRACT.md` § 4.3) `Display`
//! ve `Error` uygulamaları elle yazılmıştır.
//!
//! Tüm hatalar kullanıcı girdisinden, dosya sisteminden veya şema uyuşmazlığından
//! türetilir; hiçbir yolda `panic!` üretilmez.

use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// NodeMind'in tüm modüllerinde döndürülen hata tipi.
#[derive(Debug)]
#[non_exhaustive]
pub enum Hata {
    /// Bir dosya veya dizin üzerinde G/Ç işlemi başarısız oldu.
    Io {
        /// Yapılmaya çalışılan işlemin kısa adı (ör. `"acma"`, `"tarama"`).
        islem: &'static str,
        /// İşlemin uygulandığı yol.
        yol: PathBuf,
        /// Altta yatan `std::io` hatası.
        kaynak: io::Error,
    },
    /// Verilen yol bir dizin değil.
    KlasorDegil {
        /// Verilen yol.
        yol: PathBuf,
    },
    /// Not adı güvenli değil: yol kaçışı, ayraç, yasak karakter veya ayrılmış ad.
    GecersizNotAdi {
        /// Kullanıcının verdiği ad.
        ad: String,
        /// Ret gerekçesinin kısa açıklaması.
        sebep: &'static str,
    },
    /// Verilen yol kasa kökünün dışına taşıyor (yol kaçışı denemesi).
    KasaDisiYol {
        /// Kullanıcının verdiği göreli yol.
        yol: String,
    },
    /// Verilen dosya zaten var; **NodeMind mevcut not dosyalarını asla üzerine yazmaz**.
    NotZatenVar {
        /// Hedef dosyanın tam yolu.
        yol: PathBuf,
    },
    /// Tarih ya da tarih aralığı çözümlenemedi.
    BozukTarih {
        /// Kullanıcının verdiği ifade.
        deger: String,
        /// Beklenen biçimin kısa açıklaması.
        beklenen: &'static str,
    },
    /// `cache/index.json` dosyasının şema sürümü bu ikilinin beklediğinden farklı.
    SemaUyusmazligi {
        /// Dosyada yazan sürüm.
        bulunan: u32,
        /// Bu ikilinin beklediği sürüm.
        beklenen: u32,
        /// İndeks dosyasının yolu.
        yol: PathBuf,
    },
    /// `cache/index.json` dosyası okunamadı veya bozuk JSON içeriyor.
    BozukIndeks {
        /// İndeks dosyasının yolu.
        yol: PathBuf,
        /// Ayrıştırıcının bildirdiği kısa açıklama.
        sebep: String,
    },
    /// Dışa aktarım hedefi (JSON / DOT / HTML) yazılamadı.
    CiktiHatasi {
        /// Hedef yol.
        yol: PathBuf,
        /// Altta yatan `std::io` hatası.
        kaynak: io::Error,
    },
    /// Arama kombinasyonu çözümlenemedi (ör. alan süzgeci ile birlikte verilen
    /// çelişkili seçenek).
    GecersizSecenek {
        /// Kullanıcının verdiği değerler.
        degerler: String,
        /// Açıklama.
        sebep: &'static str,
    },
}

impl fmt::Display for Hata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Hata::Io { islem, yol, kaynak } => {
                write!(f, "{islem} basarisiz: {} ({})", yol.display(), kaynak)
            }
            Hata::KlasorDegil { yol } => write!(
                f,
                "bu yol bir klasor degil: {} (kasa = not klasoru)",
                yol.display()
            ),
            Hata::GecersizNotAdi { ad, sebep } => {
                write!(f, "gecersiz not adi: {ad:?} ({sebep})")
            }
            Hata::KasaDisiYol { yol } => write!(
                f,
                "kasa disi yol reddedildi: {yol:?} (goreli yol .. ile disari cikamaz)"
            ),
            Hata::NotZatenVar { yol } => write!(
                f,
                "not zaten var, UZERINE YAZILMADI: {} (mevcut not dosyalari asla degistirilmez)",
                yol.display()
            ),
            Hata::BozukTarih { deger, beklenen } => {
                write!(f, "tarih cozulemedi: {deger:?} (beklenen: {beklenen})")
            }
            Hata::SemaUyusmazligi {
                bulunan,
                beklenen,
                yol,
            } => write!(
                f,
                "indeks semasi uyusmuyor: {} (dosyada {bulunan}, beklenen {beklenen}); \
                 'nodemind index' ile yeniden kurun",
                yol.display()
            ),
            Hata::BozukIndeks { yol, sebep } => write!(
                f,
                "indeks dosyasi okunamadi: {} ({sebep}); 'nodemind index' ile yeniden kurun",
                yol.display()
            ),
            Hata::CiktiHatasi { yol, kaynak } => {
                write!(f, "cikti yazilamadi: {} ({})", yol.display(), kaynak)
            }
            Hata::GecersizSecenek { degerler, sebep } => {
                write!(f, "gecersiz secenek kombinasyonu: {degerler} ({sebep})")
            }
        }
    }
}

impl Error for Hata {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Hata::Io { kaynak, .. } | Hata::CiktiHatasi { kaynak, .. } => Some(kaynak),
            _ => None,
        }
    }
}

/// `Hata::Io` üreten yardımcı: yol taşımayı çağıran taraftan gizler.
pub fn io_hata(islem: &'static str, yol: &std::path::Path, kaynak: io::Error) -> Hata {
    Hata::Io {
        islem,
        yol: yol.to_path_buf(),
        kaynak,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn io_hatasi_yolu_kaynagi_gosterir() {
        let h = io_hata(
            "acma",
            std::path::Path::new("a.md"),
            io::Error::new(io::ErrorKind::NotFound, "yok"),
        );
        let m = h.to_string();
        assert!(m.contains("acma") && m.contains("a.md") && m.contains("yok"));
        assert!(h.source().is_some());
    }

    #[test]
    fn klasor_degil_mesaji_klasor_ister() {
        let h = Hata::KlasorDegil {
            yol: PathBuf::from("notlar.md"),
        };
        assert!(h.to_string().contains("klasor"));
    }

    #[test]
    fn gecersiz_ad_mesaji_nedeni_yazar() {
        let h = Hata::GecersizNotAdi {
            ad: "../kacis".to_string(),
            sebep: "yol ayraci iceriyor",
        };
        let m = h.to_string();
        assert!(m.contains("../kacis") && m.contains("yol ayraci"));
    }

    #[test]
    fn kasa_disi_yol_mesaji_aciklama_verir() {
        let h = Hata::KasaDisiYol {
            yol: "../../etc".to_string(),
        };
        assert!(h.to_string().contains("kasa disi"));
    }

    #[test]
    fn not_zaten_var_mesaji_yazma_yapilmadigini_soyler() {
        let h = Hata::NotZatenVar {
            yol: PathBuf::from("var.md"),
        };
        let m = h.to_string();
        assert!(m.contains("UZERINE YAZILMADI"));
        assert!(h.source().is_none());
    }

    #[test]
    fn bozuk_tarih_mesaji_beklenen_bicimi_soyler() {
        let h = Hata::BozukTarih {
            deger: "32-13-2026".to_string(),
            beklenen: "YYYY-AA-GG",
        };
        assert!(h.to_string().contains("YYYY-AA-GG"));
    }

    #[test]
    fn sema_uyusmazligi_her_iki_surumu_yazar() {
        let h = Hata::SemaUyusmazligi {
            bulunan: 7,
            beklenen: 1,
            yol: PathBuf::from("cache/index.json"),
        };
        let m = h.to_string();
        assert!(m.contains('7') && m.contains("beklenen 1"));
        assert!(m.contains("nodemind index"));
    }

    #[test]
    fn bozuk_indeks_mesaji_sebebi_yazar() {
        let h = Hata::BozukIndeks {
            yol: PathBuf::from("cache/index.json"),
            sebep: "beklenen `:`".to_string(),
        };
        assert!(h.to_string().contains("beklenen `:`"));
    }

    #[test]
    fn cikti_hatasi_kaynaga_zincirlenir() {
        let h = Hata::CiktiHatasi {
            yol: PathBuf::from("g.dot"),
            kaynak: io::Error::other("dolu"),
        };
        assert!(h.source().is_some());
        assert!(h.to_string().contains("g.dot"));
    }

    #[test]
    fn gecersiz_secenek_mesaji_degerleri_yazar() {
        let h = Hata::GecersizSecenek {
            degerler: "--not --blok".to_string(),
            sebep: "ikisi ayni anda verilemez",
        };
        let m = h.to_string();
        assert!(m.contains("--not --blok") && m.contains("ikisi"));
    }
}
