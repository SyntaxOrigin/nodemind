//! Testlerde geçici dosya ve dizin üreten yardımcı.
//!
//! Neden ayrı bir modül: `tempfile` crate'i bağımlılık politikasında yasaktır
//! (`WORKER_CONTRACT.md` § 3.2-F); bu yardımcı aynı sözleşmenin § 5.3'ünde
//! tarif ettiği deseni uygular.
//!
//! Yalnızca `#[cfg(test)]` altında derlenir ve `pub(crate)` olduğu için üretim
//! ikilisinde görünmez.

use std::path::{Path, PathBuf};

/// Test içinde geçici dosya/dizin üreten, `Drop` ile temizleyen kapsayıcı.
pub struct GeciciDizin {
    yol: PathBuf,
}

impl GeciciDizin {
    /// `std::env::temp_dir()` altında, etiketten türetilmiş bir dizin oluşturur.
    ///
    /// Benzersizlik `std::process::id()` ile sağlanır (rastgelelik crate'i yok).
    /// Aynı etiketle ikinci kez çağrılırsa eski içerik önce silinir; bu, testin
    /// yeniden çalıştırılması hâlinde yarım kalmış dosya bırakmaz.
    pub fn yeni(etiket: &str) -> std::io::Result<Self> {
        let kok = std::env::temp_dir().join(format!("{etiket}-{}", std::process::id()));
        // Temizlik hatası bilinçli olarak yutulur: `Drop` içinden hata
        // döndürülemez ve yarım kalmış bir dizin yeniden oluşturulabilir.
        let _ = std::fs::remove_dir_all(&kok);
        std::fs::create_dir_all(&kok)?;
        Ok(Self { yol: kok })
    }

    /// Dizin içine göreli yol döndürür.
    pub fn yol(&self) -> &Path {
        &self.yol
    }
}

impl Drop for GeciciDizin {
    fn drop(&mut self) {
        // `Drop` içinden hata döndürülemez; temizleme başarısız olsa da testi
        // düşürmemelidir. Bu, sözleşmenin "sessiz yutma" yasağına yegdir ve
        // README'de belgelenmiştir.
        let _ = std::fs::remove_dir_all(&self.yol);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn dizin_olusur_ve_temizlenir() {
        let yol = {
            let d = GeciciDizin::yeni("nodemind-yardimci").unwrap();
            assert!(d.yol().is_dir());
            let ic = d.yol().join("a.md");
            std::fs::write(&ic, b"x").unwrap();
            d.yol().to_path_buf()
        };
        assert!(!yol.exists());
    }

    #[test]
    fn ayni_etiket_ikinci_kez_acilirsa_icerik_temizlenir() {
        let d = GeciciDizin::yeni("nodemind-yardimci-tekrar").unwrap();
        let eski = d.yol().join("eski.md");
        std::fs::write(&eski, b"x").unwrap();
        drop(d);
        let d2 = GeciciDizin::yeni("nodemind-yardimci-tekrar").unwrap();
        assert!(!d2.yol().join("eski.md").exists());
    }
}
