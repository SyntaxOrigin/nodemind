//! DüğümKafa (NodeMind) çekirdek kütüphanesi.
//!
//! Amaç: kullanıcının **düz Markdown dosyaları** üzerinde blok referanslı bir
//! bilgi grafiği kurmak; notların biçimini değiştirmeden arama, ters bağlantı,
//! günlük akışı ve grafik üretmek.
//!
//! Katmanlar tek yönlü bağımlılık gösterir (rapor b06):
//!
//! ```text
//! hata  <-  kimlik  <-  markdown  <-  kasa  <-  cozum  <-  indeks  <-  graf
//!                                  |         |            |         |
//!                              gunluk <-------+------------+         |
//!                                   \---------- sema <---------------+
//! ```
//!
//! Arayüz katmanı (`main.rs`) çekirdeğin yalnızca komutlarını çağırır; dosya
//! biçimi ayrıntılarını bilmez.
//!
//! # Veri güvenliği
//!
//! NodeMind **not dosyalarına asla yazmaz**. Tek yazma noktası
//! [`Kasa::yeni_not`]'tir ve o da yalnızca **yeni** bir dosya oluşturur; hedef
//! varsa [`Hata::NotZatenVar`] döner. Türetilmiş veri (`cache/index.json`)
//! silinebilir.
//!
//! # Güvenlik ve bağımlılıklar
//!
//! Tüm kaynak `#![forbid(unsafe_code)]` ile derlenir. Ağ, pencere arayüzü,
//! grafik kütüphanesi, FTS motoru ve tarih kütüphanesi **kullanılmaz**
//! (bkz. `WORKER_CONTRACT.md` § 3.2-F ve § 3.2-G).

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::unwrap_used, clippy::expect_used)]

pub mod cozum;
pub mod graf;
pub mod gunluk;
pub mod hata;
pub mod indeks;
pub mod kasa;
pub mod kimlik;
pub mod markdown;
pub mod referans;
pub mod sema;

#[cfg(test)]
mod test_yardimcisi;

use std::collections::BTreeMap;

use crate::graf::Graf;
use crate::hata::Hata;
use crate::indeks::TersIndeks;
use crate::kasa::Kasa;
use crate::sema::{BlokKaydi, IndeksBelge, NotKaydi};

/// Kasadan **türetilmiş** arama indeksini kurar.
///
/// Bu saf bir dönüşümdür: hiçbir diske yazmaz, yalnızca bellekte bir
/// [`IndeksBelge`] üretir. Yazma işi çağıran taraftadır ([`IndeksBelge::kaydet`]).
pub fn indeks_kur(kasa: &Kasa, olusturma: u64) -> IndeksBelge {
    let mut belge = IndeksBelge::yeni(olusturma);
    let mut terimler = TersIndeks::yeni();
    let mut blok_sayisi = 0usize;

    for not in &kasa.notlar {
        belge.notlar.push(NotKaydi {
            id: not.id.clone(),
            yol: not.yol.clone(),
            ad: not.ad.clone(),
            baslik: not.baslik.clone(),
            tarih: not.tarih.map(|t| t.iso_metni()),
            degisiklik: not.degisiklik,
        });
        for ozet in not.duz_bloklar() {
            // Not adı da aranabilir olsun: ad + başlık metni indekslenir.
            let aranacak = format!("{}\n{}", not.baslik, ozet.metin);
            terimler.ekle(&ozet.kimlik, &aranacak);
            belge.bloklar.push(BlokKaydi {
                id: ozet.kimlik.clone(),
                not_id: not.id.clone(),
                baslik: not.baslik.clone(),
                not_yol: not.yol.clone(),
                baslik_yolu: ozet.baslik_yolu.clone(),
                tip: ozet.tip,
                ozet: sema::ozet_uret(&ozet.metin, 160),
                satir: ozet.satir,
            });
            blok_sayisi += 1;
        }
    }

    belge.not_sayisi = belge.notlar.len();
    belge.blok_sayisi = blok_sayisi;
    belge.terimler = terimler.terimler_disa();
    belge
}

/// Kasadaki bağlantı raporunu üretir (kırık, belirsiz, kendine bağlantılar).
pub fn baglanti_raporu(kasa: &Kasa) -> referans::Rapor {
    cozum::rapor_olustur(kasa)
}

/// Kasadan bilgi grafiğini üretir.
pub fn graf_olustur(kasa: &Kasa, secim: graf::DugumSecimi) -> Graf {
    Graf::olustur(kasa, secim)
}

/// Bir indeks belgesini bellekte aramaya hazırlar.
///
/// Şema denetimi burada bir kez daha yapılır: `IndeksBelge::yukle` dosya
/// düzeyinde denetler, bu fonksiyon **programatik** olarak üretilmiş belgeler
/// için aynı güvenceyi sağlar.
pub fn indeks_hazirla(belge: &IndeksBelge) -> Result<IndeksBelge, Hata> {
    if !belge.uyumlu_mu() {
        return Err(Hata::SemaUyusmazligi {
            bulunan: belge.sema,
            beklenen: sema::SEMA_SURUMU,
            yol: std::path::PathBuf::from("<bellek>"),
        });
    }
    Ok(belge.clone())
}

/// İndeks belgesinden blok haritasını üretir (arama için).
pub fn blok_haritasi(belge: &IndeksBelge) -> BTreeMap<String, BlokKaydi> {
    belge.blok_haritasi()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::test_yardimcisi::GeciciDizin;

    fn yaz(d: &std::path::Path, goreli: &str, icerik: &str) {
        let yol = d.join(goreli);
        if let Some(ust) = yol.parent() {
            std::fs::create_dir_all(ust).unwrap();
        }
        std::fs::write(yol, icerik).unwrap();
    }

    #[test]
    fn indeks_kur_uyumlu_belge_dondurur() {
        let d = GeciciDizin::yeni("nodemind-indeks").unwrap();
        yaz(d.yol(), "a.md", "# A\n\n- bir\n- iki\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let b = indeks_kur(&k, 42);
        assert!(b.uyumlu_mu());
        assert_eq!(b.olusturma, 42);
        assert_eq!(b.not_sayisi, 1);
        assert_eq!(b.blok_sayisi, 3);
    }

    #[test]
    fn indeks_kur_not_adini_da_arama_yapar() {
        let d = GeciciDizin::yeni("nodemind-indeks-ad").unwrap();
        yaz(d.yol(), "Kavramlar.md", "# Kavramlar\n\nBir paragraf.\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let b = indeks_kur(&k, 0);
        let t = TersIndeks::yeni();
        drop(t);
        assert!(b.terimler.contains_key("kavramlar"));
    }

    #[test]
    fn indeks_kur_bos_kasada_bos_belge_dondurur() {
        let d = GeciciDizin::yeni("nodemind-indeks-bos").unwrap();
        let k = Kasa::ac(d.yol()).unwrap();
        let b = indeks_kur(&k, 0);
        assert_eq!(b.not_sayisi, 0);
        assert_eq!(b.blok_sayisi, 0);
        assert!(b.terimler.is_empty());
    }

    #[test]
    fn ayni_kasadan_iki_indeks_bayt_bayt_ayni() {
        let d = GeciciDizin::yeni("nodemind-indeks-kararli").unwrap();
        yaz(d.yol(), "a.md", "# A\n\n- bir\n- iki\n");
        yaz(d.yol(), "b.md", "# B\n\n[[a]] baglanti.\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let a = indeks_kur(&k, 7).metin().unwrap();
        let b = indeks_kur(&k, 7).metin().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn indeks_hazirla_uyumsuz_semayi_reddeder() {
        let d = GeciciDizin::yeni("nodemind-indeks-hazirla").unwrap();
        yaz(d.yol(), "a.md", "# A\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let mut b = indeks_kur(&k, 0);
        b.sema = 0;
        let h = indeks_hazirla(&b).unwrap_err();
        assert!(h.to_string().contains("semasi uyusmuyor"));
    }

    #[test]
    fn graf_olustur_bos_kasada_bos_graf_dondurur() {
        let d = GeciciDizin::yeni("nodemind-graf-bos").unwrap();
        let k = Kasa::ac(d.yol()).unwrap();
        let g = graf_olustur(&k, graf::DugumSecimi::Hepsi);
        assert_eq!(g.dugum_sayisi(), 0);
        assert!(g.ascii().contains("graf bos"));
    }

    #[test]
    fn baglanti_raporu_bos_kasada_bos() {
        let d = GeciciDizin::yeni("nodemind-rapor-bos").unwrap();
        let k = Kasa::ac(d.yol()).unwrap();
        assert_eq!(baglanti_raporu(&k).toplam(), 0);
    }

    #[test]
    fn blok_haritasi_indeks_belgesinden_uretilir() {
        let d = GeciciDizin::yeni("nodemind-blokharita").unwrap();
        yaz(d.yol(), "a.md", "# A\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let b = indeks_kur(&k, 0);
        let m = blok_haritasi(&b);
        assert_eq!(m.len(), 1);
    }
}
