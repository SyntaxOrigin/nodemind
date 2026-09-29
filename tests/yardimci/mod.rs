//! Entegrasyon testlerinde ortak kullanılan geçici kasa yardımcısı.
//!
//! `tempfile` crate'i bağımlılık politikasında yasaktır
//! (`WORKER_CONTRACT.md` § 3.2-F); bu yardımcı sözleşmenin § 5.3'ünde tarif
//! ettiği deseni uygular ve `Drop` ile temizlik yapar.
//!
//! Ayrıca `std::fs::write` ile dosya yazılır; `Set-Content -Encoding UTF8` gibi
//! araçlar Windows'ta BOM ekleyip testleri kırabiliyordu.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Test içinde geçici not klasörü üreten, `Drop` ile temizleyen kapsayıcı.
pub struct GeciciKasa {
    yol: PathBuf,
}

impl GeciciKasa {
    /// `std::env::temp_dir()` altında, etiketten türetilmiş benzersiz bir dizin
    /// oluşturur.
    ///
    /// Benzersizlik `std::process::id()` ile sağlanır (rastgelelik crate'i yok).
    /// Aynı etiketle ikinci çağrıda eski içerik önce silinir; testlerin
    /// eş zamanlı çalışması bu yüzden **etiket ayrımı** gerektirir.
    pub fn yeni(etiket: &str) -> GeciciKasa {
        let kok = std::env::temp_dir().join(format!("nodemind-it-{etiket}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&kok);
        std::fs::create_dir_all(&kok).expect("gecici kasa olusturulamadi");
        GeciciKasa { yol: kok }
    }

    /// Kasanın kök dizini.
    pub fn yol(&self) -> &Path {
        &self.yol
    }

    /// Kasanın kök dizinini `&str` olarak döndürür (alt komut argümanı için).
    pub fn yol_str(&self) -> &str {
        self.yol.to_str().expect("gecici yol ASCII olmali")
    }

    /// Kasa altına göreli bir not yazar.
    pub fn yaz(&self, goreli: &str, icerik: &str) {
        let yol = self.yol.join(goreli);
        if let Some(ust) = yol.parent() {
            std::fs::create_dir_all(ust).expect("alt dizin olusturulamadi");
        }
        std::fs::write(&yol, icerik).expect("not yazilamadi");
    }

    /// Kasa altından göreli bir dosyanın içeriğini okur.
    pub fn oku(&self, goreli: &str) -> String {
        std::fs::read_to_string(self.yol.join(goreli)).expect("not okunamadi")
    }
}

impl Drop for GeciciKasa {
    fn drop(&mut self) {
        // `Drop` içinden hata döndürülemez; temizleme başarısız olsa da testi
        // düşürmemelidir. Sözleşmenin "sessiz yutma" yasağına yegdir.
        let _ = std::fs::remove_dir_all(&self.yol);
    }
}

/// Derlenmiş `nodemind` ikilisini kasa verilen şekilde çalıştırır.
///
/// İkili, `cargo test` sırasında `CARGO_BIN_EXE_<ad>` ile mutlak yolda verilir;
/// bu, `target/debug` yolunu tahmin etmeye gerek bırakmaz.
pub fn calistir(kasa: &GeciciKasa, args: &[&str]) -> Output {
    let cikti = Command::new(env!("CARGO_BIN_EXE_nodemind"))
        .arg("--kasa")
        .arg(kasa.yol())
        .args(args)
        .output()
        .expect("nodemind ikilisi calistirilamadi");
    Output {
        kod: cikti.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&cikti.stdout).to_string(),
        stderr: String::from_utf8_lossy(&cikti.stderr).to_string(),
    }
}

/// Bir alt komutun yakalanmış çıktısı.
#[derive(Debug)]
pub struct Output {
    /// İşlem çıkış kodu.
    pub kod: i32,
    /// Standart çıktı.
    pub stdout: String,
    /// Hata çıktısı.
    pub stderr: String,
}

impl Output {
    /// Standart ve hata çıktısının birleşimi (kullanıcının gördüğü her şey).
    pub fn tum(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }

    /// Çıktının belirli bir metni içerdiğini doğrular; değilse test düşer.
    pub fn icermeli(&self, beklenen: &str) -> &Self {
        assert!(
            self.tum().contains(beklenen),
            "beklenen `{beklenen}` yok.\n--- stdout ---\n{}\n--- stderr ---\n{}",
            self.stdout,
            self.stderr
        );
        self
    }

    /// Çıktının bir metni **içermediğini** doğrular.
    pub fn icermemeli(&self, beklenmeyen: &str) -> &Self {
        assert!(
            !self.tum().contains(beklenmeyen),
            "beklenmeyen `{beklenmeyen}` var.\n--- stdout ---\n{}",
            self.stdout
        );
        self
    }
}
