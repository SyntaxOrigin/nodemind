//! Kararlı blok kimliği üretimi.
//!
//! Sorumluluğu, bir bloğun içeriğinden ve not içindeki konumundan **deterministik**
//! bir kimlik türetmektir. Kimlik dosyaya yazılmaz; her `index`/`check`/`graph`
//! çağrısında yeniden hesaplanır ve aynı girdiden daima aynı sonuç çıkar.
//!
//! Neden içerikten türetiyoruz (MANIFEST kart 21, madde 2): ayrıştırıcı değişse de
//! not dosyaları düz metin olarak okunabilir kalır ve kullanıcının diski ile
//! uygulamanın sürümü arasında bir bağımlılık oluşmaz.
//!
//! Bilinen ve **kasıtlı** yan etki: bir bloğun metni ya da konumu değişirse
//! kimliği de değişir, dolayısıyla o bloğa yazılmış `((kimlik))` referansları kırık
//! hâle gelir. Bu, sessizce yanlış bloğa bağlanmaktansa dürüst bir davranıştır
//! ve README'nin "Bilinen Sınırlamalar" bölümünde belgelenmiştir.

/// Ayraç: kimlik girdisindeki alanları birbirinden ayırmak için kullanılır.
///
/// Neden `0x1F` (Unit Separator): Markdown metninde ve dosya yollarında
/// geçmeyen tek bir kontrol baytı; alan çakışmasını imkânsız kılar.
const ALAN_AYRACI: u8 = 0x1F;

/// FNV-1a 64 bit karmasının başlangıç değeri (offset basis).
const FNV_BASLANGIC: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64 bit karmasının asal çarpanı (prime).
const FNV_CARPAN: u64 = 0x0000_0100_0000_01b3;

/// NodeMind üretiminde kullanılan blok kimliği uzunluğu (onaltılık karakter).
const KIMLIK_UZUNLUGU: usize = 12;

/// Ham girdi baytlarının FNV-1a 64 bit karmasını hesaplar.
///
/// Kütüphane kullanılmaz (`WORKER_CONTRACT.md` § 3.2); FNV-1a tek geçişli,
/// çarpma tabanlı, çakışma davranışı iyi bilinen bir özet fonksiyondur ve burada
/// yalnızca **kısa ve kararlı** işaretçi üretmek için kullanılır — güvenlik
/// amaçlı değildir.
pub fn fnv1a64(baytlar: &[u8]) -> u64 {
    let mut karma = FNV_BASLANGIC;
    for bayt in baytlar {
        karma ^= u64::from(*bayt);
        karma = karma.wrapping_mul(FNV_CARPAN);
    }
    karma
}

/// Onaltılık ve sıfır dolgulu gösterim (`to_string` ile aynı sonucu verir).
fn onaltilik(deger: u64) -> String {
    let mut s = format!("{deger:016x}");
    s.truncate(KIMLIK_UZUNLUGU);
    s
}

/// Bir bloğun ham kimlik girdisini üretir.
///
/// Alanlar `0x1F` ile ayrılır: `not_id`, blok yolu, blok tipi adı, metin.
fn kimlik_girdisi(not_id: &str, yol: &str, tip: &str, metin: &str) -> String {
    let mut girdi = String::with_capacity(not_id.len() + yol.len() + tip.len() + metin.len() + 3);
    girdi.push_str(not_id);
    girdi.push(ALAN_AYRACI as char);
    girdi.push_str(yol);
    girdi.push(ALAN_AYRACI as char);
    girdi.push_str(tip);
    girdi.push(ALAN_AYRACI as char);
    girdi.push_str(metin);
    girdi
}

/// `not_id`, konum ve metinden kararlı bir blok kimliği üretir.
///
/// `yol`, not ağacındaki konum yoludur (kök düğümler `"0"`, `"1"`, ...; çocuklar
/// `"<ebeveyn>.<sira>"` biçiminde). Aynı girdi her zaman aynı kimliği verir.
pub fn blok_kimligi(not_id: &str, yol: &str, tip: &str, metin: &str) -> String {
    onaltilik(fnv1a64(kimlik_girdisi(not_id, yol, tip, metin).as_bytes()))
}

/// `not_id` ve blok yolundan bir **not kimliği** (dosya kimliği) üretir.
///
/// Not kimlikleri dosyanın göreli yolundan türetilir; içerik değişse de not kimliği
/// değişmez, böylece `cache/index.json` yeniden yazıldığında eski bağlantılar
/// geçerli kalır.
pub fn not_kimligi(goreli_yol: &str) -> String {
    onaltilik(fnv1a64(goreli_yol.as_bytes()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_bos_girdi_baslangic_degerini_verir() {
        assert_eq!(fnv1a64(b""), FNV_BASLANGIC);
    }

    #[test]
    fn fnv1a_kisa_girdi_karsilastirilabilir_deger_verir() {
        // FNV-1a 64: "a" -> 0xaf63dc4c8601ec8c
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        // FNV-1a 64: "foobar" -> 0x85944171f73967e8
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn onaltilik_on_iki_hane_verir() {
        let s = onaltilik(0x0123_4567_89ab_cdef);
        assert_eq!(s.len(), KIMLIK_UZUNLUGU);
        assert_eq!(s, "0123456789ab");
    }

    #[test]
    fn blok_kimligi_kararli_ve_onaltilik() {
        let a = blok_kimligi("not.md", "0", "Baslik", "Giris");
        let b = blok_kimligi("not.md", "0", "Baslik", "Giris");
        assert_eq!(a, b);
        assert_eq!(a.len(), KIMLIK_UZUNLUGU);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn blok_kimligi_not_degisince_degisir() {
        let a = blok_kimligi("a.md", "0", "Baslik", "Giris");
        let b = blok_kimligi("b.md", "0", "Baslik", "Giris");
        assert_ne!(a, b);
    }

    #[test]
    fn blok_kimligi_yol_degisince_degisir() {
        let a = blok_kimligi("a.md", "0", "Baslik", "Giris");
        let b = blok_kimligi("a.md", "0.1", "Baslik", "Giris");
        assert_ne!(a, b);
    }

    #[test]
    fn blok_kimligi_metin_degisince_degisir() {
        let a = blok_kimligi("a.md", "0", "Baslik", "Giris");
        let b = blok_kimligi("a.md", "0", "Baslik", "Cikis");
        assert_ne!(a, b);
    }

    #[test]
    fn blok_kimligi_tip_degisince_degisir() {
        let a = blok_kimligi("a.md", "0", "Baslik", "X");
        let b = blok_kimligi("a.md", "0", "Liste", "X");
        assert_ne!(a, b);
    }

    #[test]
    fn alan_ayraci_alan_karisikligini_engeller() {
        // "ab" + "c" ile "a" + "bc" ayni girdi olmamali.
        let a = blok_kimligi("ab", "c", "T", "m");
        let b = blok_kimligi("a", "bc", "T", "m");
        assert_ne!(a, b);
    }

    #[test]
    fn not_kimligi_yoldan_turetilir_ve_kararlidir() {
        let a = not_kimligi("gunluk/2026-09-29.md");
        let b = not_kimligi("gunluk/2026-09-29.md");
        assert_eq!(a, b);
        assert_ne!(a, not_kimligi("gunluk/2026-09-30.md"));
    }

    #[test]
    fn not_kimligi_ic_ice_yolla_kararli() {
        assert!(!not_kimligi("a/b/c.md").is_empty());
        assert_ne!(not_kimligi("a/b/c.md"), not_kimligi("a/bc.md"));
    }
}
