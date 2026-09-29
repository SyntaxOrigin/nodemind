//! Bağlantı çözümleme uygulaması: kasa → düzce → çözüm raporu ve ters bağlantılar.
//!
//! Bu modül [`crate::kasa`] ile [`crate::referans`] arasındaki köprüdür ve
//! bağımlılık yönünü tek yönlü tutar: `kasa` ve `referans` bu modülü bilmez.
//!
//! Üç çıktı üretilir:
//!
//! * [`duzce_olustur`] — `[[...]]` / `((...))` çözümlemesi için dizin,
//! * [`rapor_olustur`] — `check` alt komutunun kırık/belirsiz listeleri,
//! * [`ters_baglantilar`] — `backlinks` alt komutunun "bana bağlı olanlar" listesi.

use std::collections::BTreeMap;

use crate::kasa::Kasa;
use crate::referans::{
    baglantilari_cikar, coz, kendine_mi, Baglanti, BaglantiTipi, BlokKaydi, Duzce, Rapor,
};

/// Kasadan çözümleme düzcesi üretir.
///
/// Not adları **iki biçimde** kaydedilir: dosya adı (uzantısız) ve kasanın köküne
/// göreli yol. Böylece hem `[[Arama]]` hem `[[gunluk/2026-09-28]]` yazımı
/// çözülür; `Kasa::not_ara` ile arama komutu da aynı iki biçimi kabul eder.
pub fn duzce_olustur(kasa: &Kasa) -> Duzce {
    let mut duzce = Duzce::yeni();
    for not in &kasa.notlar {
        duzce.not_ekle(&not.ad, &not.id);
        // Yol biçimi yalnızca ada **farklıysa** eklenir; kök düzeydeki bir notta
        // `Arama.md` ve `Arama` aynı anahtardır ve aynı not iki kez
        // kaydedilirse (yanlışlıkla) her bağlantı belirsiz görünürdü.
        let yol = yol_uzantisiz(&not.yol);
        if yol != not.ad {
            duzce.not_ekle(&yol, &not.id);
        }
        for ozet in not.duz_bloklar() {
            duzce.blok_ekle(
                &ozet.kimlik,
                BlokKaydi::yeni(&not.id, &ozet.baslik_yolu, &ozet.metin, ozet.satir),
            );
        }
    }
    duzce
}

/// Bir göreli yoldan `.md` / `.markdown` uzantısını çıkarır.
///
/// `[[gunluk/2026-09-28]]` yazımı uzantısız yazılır; uzantısız biçim burada
/// üretilerek düzceye eklenir.
fn yol_uzantisiz(yol: &str) -> String {
    for uzanti in [".md", ".markdown"] {
        if let Some(kisa) = yol.strip_suffix(uzanti) {
            return kisa.to_string();
        }
    }
    yol.to_string()
}

/// Kasadaki tüm bağlantıları çözümleyip rapor üretir.
pub fn rapor_olustur(kasa: &Kasa) -> Rapor {
    let duzce = duzce_olustur(kasa);
    let mut rapor = Rapor::default();
    for not in &kasa.notlar {
        for ozet in not.duz_bloklar() {
            for baglanti in baglantilari_cikar(&ozet.metin) {
                let c = coz(&baglanti, &not.id, &ozet.kimlik, &duzce);
                if kendine_mi(&c, &duzce) {
                    rapor.kendine.push(c.clone());
                }
                rapor.ekle(c);
            }
        }
    }
    rapor
}

/// Tek bir kaynak bağlantısı (ters bağlantı çıktısı için).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GelenBaglanti {
    /// Bağlantıyı yapan bloğun kimliği.
    pub kaynak_blok: String,
    /// Bağlantıyı yapan notun yolu.
    pub kaynak_not: String,
    /// Kaynak notun okunabilir başlığı.
    pub kaynak_baslik: String,
    /// Kaynak bloğun başlık yolu.
    pub blok_baslik_yolu: String,
    /// Bağlantının lehçesi.
    pub tip: BaglantiTipi,
    /// Ham hedef metni.
    pub hedef: String,
    /// 1 tabanlı satır numarası.
    pub satir: usize,
}

/// Ters bağlantı haritası: hedef düğüm → o düğüme bağlantı veren kaynaklar.
///
/// Kilit noktası: bir not adı **birden çok** nota karşılık geliyorsa
/// ([[ yinelenen ad ]] durumu) o ad için hiçbir ters bağlantı üretilmez; çünkü
/// hangi notun kastedildiği belirsizdir. Kullanıcı bu durumu `check`
/// çıktısında görür ve adı benzersizleştirir.
pub fn ters_baglantilar(kasa: &Kasa) -> BTreeMap<String, Vec<GelenBaglanti>> {
    let duzce = duzce_olustur(kasa);
    let mut harita: BTreeMap<String, Vec<GelenBaglanti>> = BTreeMap::new();
    for not in &kasa.notlar {
        for ozet in not.duz_bloklar() {
            for baglanti in baglantilari_cikar(&ozet.metin) {
                let Some(anahtar) = hedef_dugum(&baglanti, &duzce) else {
                    // Kırık ve belirsiz bağlantıların ters bağlantısı yoktur.
                    continue;
                };
                harita.entry(anahtar).or_default().push(GelenBaglanti {
                    kaynak_blok: ozet.kimlik.clone(),
                    kaynak_not: not.yol.clone(),
                    kaynak_baslik: not.baslik.clone(),
                    blok_baslik_yolu: ozet.baslik_yolu.clone(),
                    tip: baglanti.tip,
                    hedef: baglanti.hedef.clone(),
                    satir: ozet.satir,
                });
            }
        }
    }
    harita
}

/// Bir bağlantının çözümlenmiş hedef düğüm kimliği (belirsiz/kırık ise `None`).
fn hedef_dugum(baglanti: &Baglanti, duzce: &Duzce) -> Option<String> {
    match baglanti.tip {
        BaglantiTipi::BlokKimligi => duzce
            .blok_ara(&baglanti.hedef)
            .map(|_| baglanti.hedef.clone()),
        BaglantiTipi::NotAdi => {
            let adaylar = duzce.not_adaylari(&baglanti.hedef);
            match adaylar.len() {
                1 => Some(adaylar[0].clone()),
                _ => None,
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::test_yardimcisi::GeciciDizin;
    use std::path::Path;

    fn yaz(d: &Path, goreli: &str, icerik: &str) {
        let yol = d.join(goreli);
        if let Some(ust) = yol.parent() {
            std::fs::create_dir_all(ust).unwrap();
        }
        std::fs::write(yol, icerik).unwrap();
    }

    /// Etiket testler arasinda benzersiz olmalidir: `GeciciDizin` dizini
    /// `std::process::id()` ile adlandirilir ve ayni etiket es zamanli iki
    /// testte birbirinin icerigini silerdi.
    fn kasa_ornek(etiket: &str) -> (GeciciDizin, Kasa) {
        let d = GeciciDizin::yeni(etiket).unwrap();
        yaz(d.yol(), "a.md", "# A\n\nBir [[B]] baglantisi.\n");
        yaz(d.yol(), "b.md", "# B\n\nGeri baglanti [[A]].\n");
        let k = Kasa::ac(d.yol()).unwrap();
        (d, k)
    }

    #[test]
    fn duzce_not_ve_bloklari_toplar() {
        let (_d, k) = kasa_ornek("nodemind-cozum-147");
        let duzce = duzce_olustur(&k);
        assert_eq!(duzce.not_aday_sayisi("a"), 1);
        assert_eq!(duzce.not_aday_sayisi("b"), 1);
        assert!(duzce.blok_sayisi() >= 4);
    }

    #[test]
    fn rapor_cozulen_baglantilari_sayar() {
        let (_d, k) = kasa_ornek("nodemind-cozum-156");
        let r = rapor_olustur(&k);
        assert_eq!(r.kirik.len(), 0);
        assert_eq!(r.belirsiz.len(), 0);
        assert_eq!(r.cozulen.len(), 2);
        assert!(!r.sorunlu_mu());
    }

    #[test]
    fn duzce_yol_bicimini_de_kabul_eder() {
        let d = GeciciDizin::yeni("nodemind-cozum-yol").unwrap();
        yaz(d.yol(), "gunluk/2026-09-28.md", "# G\n");
        yaz(d.yol(), "kok.md", "# K\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let duzce = duzce_olustur(&k);
        // Hem `[[2026-09-28]]` hem `[[gunluk/2026-09-28]]` cozulur.
        assert_eq!(duzce.not_aday_sayisi("2026-09-28"), 1);
        assert_eq!(duzce.not_aday_sayisi("gunluk/2026-09-28"), 1);
        // Kok duzeydeki not iki kez kaydedilmez: ad = yol.
        assert_eq!(duzce.not_aday_sayisi("kok"), 1);
    }

    #[test]
    fn rapor_kirk_baglantiyi_listeler() {
        let d = GeciciDizin::yeni("nodemind-cozum-kirik").unwrap();
        yaz(d.yol(), "a.md", "# A\n\n[[Olmayan]] ve ((yokkimlik)).\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let r = rapor_olustur(&k);
        assert_eq!(r.kirik.len(), 2);
        assert!(r.sorunlu_mu());
        assert!(!r.kirik[0].hedef.is_empty());
    }

    #[test]
    fn rapor_yinelenen_adi_belirsiz_olarak_listeler() {
        let d = GeciciDizin::yeni("nodemind-cozum-belirsiz").unwrap();
        yaz(d.yol(), "bir/not.md", "# 1\n");
        yaz(d.yol(), "iki/not.md", "# 2\n");
        yaz(d.yol(), "uc.md", "# 3\n\nBaglanti [[not]].\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let r = rapor_olustur(&k);
        assert_eq!(r.belirsiz.len(), 1);
        assert!(r.sorunlu_mu());
    }

    #[test]
    fn rapor_kendine_baglantiyi_ayrica_toplar() {
        let d = GeciciDizin::yeni("nodemind-cozum-kendine").unwrap();
        yaz(d.yol(), "a.md", "# A\n\nKendine baglanti [[a]].\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let r = rapor_olustur(&k);
        assert_eq!(r.kendine.len(), 1);
        assert_eq!(r.cozulen.len(), 1);
    }

    #[test]
    fn ters_baglantilar_not_bazinda_calisir() {
        let (_d, k) = kasa_ornek("nodemind-cozum-199");
        let h = ters_baglantilar(&k);
        let b = k.not_ara("b")[0];
        assert_eq!(h.get(&b.id).unwrap().len(), 1);
        assert_eq!(h.get(&b.id).unwrap()[0].kaynak_baslik, "A");
    }

    #[test]
    fn ters_baglantilar_bos_kasada_bos_donulur() {
        let d = GeciciDizin::yeni("nodemind-cozum-bos").unwrap();
        let k = Kasa::ac(d.yol()).unwrap();
        assert!(ters_baglantilar(&k).is_empty());
        assert_eq!(rapor_olustur(&k).toplam(), 0);
    }

    #[test]
    fn ters_baglantilar_belirsiz_adi_atlar() {
        let d = GeciciDizin::yeni("nodemind-cozum-belirsiz-ters").unwrap();
        yaz(d.yol(), "bir/not.md", "# 1\n");
        yaz(d.yol(), "iki/not.md", "# 2\n");
        yaz(d.yol(), "uc.md", "# 3\n\nBaglanti [[not]].\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let h = ters_baglantilar(&k);
        assert!(h.values().all(|v| v.is_empty()));
    }

    #[test]
    fn ters_baglantilar_blok_hedefini_cozumler() {
        let d = GeciciDizin::yeni("nodemind-cozum-blok").unwrap();
        yaz(d.yol(), "a.md", "# Hedef Baslik\n");
        let k = Kasa::ac(d.yol()).unwrap();
        let hedef = k.not_ara("a")[0].duz_bloklar()[0].kimlik.clone();
        yaz(
            d.yol(),
            "b.md",
            &format!("# B\n\nKimlige bagli (({})).\n", hedef),
        );
        let k2 = Kasa::ac(d.yol()).unwrap();
        let h = ters_baglantilar(&k2);
        assert_eq!(h.get(&hedef).unwrap().len(), 1);
        assert_eq!(h.get(&hedef).unwrap()[0].tip, BaglantiTipi::BlokKimligi);
    }

    #[test]
    fn gelen_baglanti_bilgileri_tamdir() {
        let d = GeciciDizin::yeni("nodemind-cozum-bilgi").unwrap();
        yaz(d.yol(), "hedef.md", "# Hedef\n");
        yaz(
            d.yol(),
            "kaynak.md",
            "# Kaynak\n\n## Bolum\n\nBagli [[hedef]].\n",
        );
        let k = Kasa::ac(d.yol()).unwrap();
        let hedef = k.not_ara("hedef")[0].id.clone();
        let h = ters_baglantilar(&k);
        let g = &h.get(&hedef).unwrap()[0];
        assert_eq!(g.kaynak_not, "kaynak.md");
        assert_eq!(g.kaynak_baslik, "Kaynak");
        assert_eq!(g.blok_baslik_yolu, "Kaynak > Bolum");
        assert_eq!(g.hedef, "hedef");
        assert!(g.satir >= 1);
        assert!(!g.kaynak_blok.is_empty());
    }
}
