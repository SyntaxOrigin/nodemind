//! Tarih hesabı ve günlük/haftalık akış.
//!
//! `chrono`/`time` crate'leri **yasaktır** (`WORKER_CONTRACT.md` § 3.2-F), bu
//! yüzden sivil tarih dönüşümü elle yazılmıştır. Kullanılan algoritma,
//! Howard Hinnant'ın "chrono-Compatible Low-Level Date Algorithms" makalesindeki
//! `days_from_civil` / `civil_from_days` yöntemidir; prolematik iki `i64`
//! bölmesi ve 400 yıllık dönem mantığı dışında bir durum içermez, test vektörleri
//! [`crate::gunluk`] birim testlerinde gömülüdür.
//!
//! Yalnızca `std::time::SystemTime` kullanılır ve yalnızca **bugünün** tarihini
//! bulmak için; testler sabit tarih kullanır (`WORKER_CONTRACT.md` § 5.2).

use std::time::{SystemTime, UNIX_EPOCH};

use crate::hata::Hata;

/// Bir gündeki saniye sayısı.
const GUN_SANIYE: u64 = 86_400;

/// Tarihin okunabilir adları (Türkçe, ASCII'ye katlanmamış hâlde saklanır).
const AY_ADLARI: [&str; 12] = [
    "Ocak", "Subat", "Mart", "Nisan", "Mayis", "Haziran", "Temmuz", "Agustos", "Eylul", "Ekim",
    "Kasim", "Aralik",
];

/// Haftanın gün adları (Pazartesi = 0).
const GUN_ADLARI: [&str; 7] = [
    "Pazartesi",
    "Sali",
    "Carsamba",
    "Persembe",
    "Cuma",
    "Cumartesi",
    "Pazar",
];

/// Sivil tarih (yıl, ay, gün).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tarih {
    /// Yıl (ISO 8601, dört haneli).
    pub yil: i64,
    /// Ay (1..=12).
    pub ay: u32,
    /// Gün (1..=31).
    pub gun: u32,
}

impl Tarih {
    /// Bugünün tarihini döndürür.
    ///
    /// `SystemTime::now()` yalnızca burada, yalnızca çıktı üretmek için
    /// çağrılır; hiçbir karar testte bu fonksiyona dayanmaz.
    pub fn bugun() -> Tarih {
        let simdi = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Tarih::epoch_gununden((simdi / GUN_SANIYE) as i64)
    }

    /// 1970-01-01'den itibaren geçen gün sayısından tarih üretir.
    pub fn epoch_gununden(gun: i64) -> Tarih {
        // Hinnant, civil_from_days: gün sayısını 400 yıllık döneme indirger.
        let z = gun + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097; // [0, 146096]
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
        let mp = (5 * doy + 2) / 153; // [0, 11]
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
        Tarih {
            yil: if m <= 2 { y + 1 } else { y },
            ay: m,
            gun: d,
        }
    }

    /// Tarihi 1970-01-01'den itibaren geçen gün sayısına çevirir.
    pub fn gun_sayisi(&self) -> i64 {
        // Hinnant, days_from_civil.
        let y = if self.ay <= 2 { self.yil - 1 } else { self.yil };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let m = self.ay as i64;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + self.gun as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// Verilen gün sayısını ekler.
    pub fn gun_ekle(&self, adet: i64) -> Tarih {
        Tarih::epoch_gununden(self.gun_sayisi() + adet)
    }

    /// Tarihin geçerli olup olmadığını denetler (ay günü sınırları dâhil).
    pub fn gecerli_mi(&self) -> bool {
        if !(1..=12).contains(&self.ay) || self.gun < 1 {
            return false;
        }
        self.gun <= self.ay_gun_sayisi()
    }

    /// Bulunulan ayın gün sayısı (Şubat artık yılı hesaba katar).
    pub fn ay_gun_sayisi(&self) -> u32 {
        match self.ay {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if self.artik_yil_mi() {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        }
    }

    /// Artık yıl kuralı: 4'e bölünür, 100'e bölünmez, 400'e bölünür.
    pub fn artik_yil_mi(&self) -> bool {
        self.yil % 4 == 0 && (self.yil % 100 != 0 || self.yil % 400 == 0)
    }

    /// `YYYY-AA-GG` biçiminde yazar.
    pub fn iso_metni(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.yil, self.ay, self.gun)
    }

    /// Dosya adı biçiminde yazar (`YYYY-AA-GG.md`).
    pub fn dosya_adi(&self) -> String {
        format!("{}.md", self.iso_metni())
    }

    /// Ayın adı.
    pub fn ay_adi(&self) -> &'static str {
        AY_ADLARI
            .get(self.ay.saturating_sub(1) as usize)
            .copied()
            .unwrap_or("?")
    }

    /// Haftanın gün adı (0 = Pazartesi).
    pub fn gun_adi(&self) -> &'static str {
        // 1970-01-1 bir Perşembedir; hmm, 1970-01-01 Perşendi'dir (doy=3).
        let karsilama = (self.gun_sayisi() + 3).rem_euclid(7) as usize;
        GUN_ADLARI[karsilama]
    }

    /// ISO 8601 hafta numarası.
    pub fn iso_hafta(&self) -> u32 {
        // ISO: hafta, yılın Perşembe gününe ait yıla aittir. Haftanın Perşembe'si
        // = Pazartesi + 3 gün, Pazartesi = bugun - (hafta_gunu - 1).
        let gun = self.gun_sayisi();
        let persembe = gun - (self.hafta_gunu() as i64) + 4;
        let yil = Tarih::epoch_gununden(persembe).yil;
        let ocak1 = Tarih { yil, ay: 1, gun: 1 };
        let ocak1_gun = ocak1.gun_sayisi();
        (((persembe - ocak1_gun) / 7) + 1) as u32
    }

    /// ISO hafta günü (1 = Pazartesi .. 7 = Pazar).
    pub fn hafta_gunu(&self) -> u32 {
        (self.gun_sayisi() + 3).rem_euclid(7) as u32 + 1
    }

    /// Verilen ISO yılı ve hafta numarasının Pazartesi'sini döndürür.
    pub fn hafta_baslangici(yil: i64, hafta: u32) -> Option<Tarih> {
        if !(1..=53).contains(&hafta) {
            return None;
        }
        let ocak4 = Tarih { yil, ay: 1, gun: 4 };
        let hafta1_pazartesi = ocak4.gun_ekle(-(ocak4.hafta_gunu() as i64 - 1));
        let t = hafta1_pazartesi.gun_ekle(((hafta as i64) - 1) * 7);
        if t.iso_hafta() == hafta && t.yil == yil {
            Some(t)
        } else {
            None
        }
    }

    /// Kullanıcı girdisini tarihe çevirir: `bugun`, `YYYY-AA-GG` veya `YYYY-AA`.
    pub fn cozumle(secim: &str) -> Result<Tarih, Hata> {
        let s = secim.trim();
        if s.eq_ignore_ascii_case("bugun") || s == "today" {
            return Ok(Tarih::bugun());
        }
        let parcalar: Vec<&str> = s.split('-').collect();
        let bozuk = || Hata::BozukTarih {
            deger: secim.to_string(),
            beklenen: "'bugun', YYYY-AA-GG veya YYYY-AA",
        };
        match parcalar.len() {
            3 => {
                if parcalar[0].len() != 4 || !parcalar[0].bytes().all(|b| b.is_ascii_digit()) {
                    return Err(bozuk());
                }
                let yil = parcalar[0].parse::<i64>().map_err(|_| bozuk())?;
                let ay = parcalar[1].parse::<u32>().map_err(|_| bozuk())?;
                let gun = parcalar[2].parse::<u32>().map_err(|_| bozuk())?;
                let t = Tarih { yil, ay, gun };
                if t.gecerli_mi() {
                    Ok(t)
                } else {
                    Err(bozuk())
                }
            }
            2 => {
                let yil = parcalar[0].parse::<i64>().map_err(|_| bozuk())?;
                let ay = parcalar[1].parse::<u32>().map_err(|_| bozuk())?;
                if !(1..=12).contains(&ay) {
                    return Err(bozuk());
                }
                Ok(Tarih { yil, ay, gun: 1 })
            }
            _ => Err(bozuk()),
        }
    }

    /// Bir dosya adının başındaki `YYYY-AA-GG` bölümünü tarihe çevirir.
    ///
    /// Günlük olmayan adlar (`rapor-2026.md`) `None` döndürür; ayrıştırma
    /// katıdır, "içinde geçiyor" değil "tam olarak biçim" arar.
    pub fn dosya_adindan(ad: &str) -> Option<Tarih> {
        let parcalar: Vec<&str> = ad.split('-').collect();
        if parcalar.len() != 3 {
            return None;
        }
        if parcalar.iter().any(|p| p.len() != 2 && p.len() != 4) {
            return None;
        }
        if parcalar[0].len() != 4 {
            return None;
        }
        let yil = parcalar[0].parse::<i64>().ok()?;
        let ay = parcalar[1].parse::<u32>().ok()?;
        let gun = parcalar[2].parse::<u32>().ok()?;
        let t = Tarih { yil, ay, gun };
        if t.gecerli_mi() {
            Some(t)
        } else {
            None
        }
    }

    /// Tarihi `29 Eylul 2026 Cuma` biçiminde yazar.
    pub fn uzun(&self) -> String {
        format!(
            "{} {} {} {}",
            self.gun,
            self.ay_adi(),
            self.yil,
            self.gun_adi()
        )
    }
}

/// Bir ISO haftasının yedi gününü Pazartesi'den Pazar'a döndürür.
pub fn hafta_gunleri(yil: i64, hafta: u32) -> Result<Vec<Tarih>, Hata> {
    let baslangic = Tarih::hafta_baslangici(yil, hafta).ok_or_else(|| Hata::BozukTarih {
        deger: format!("{yil}-W{hafta:02}"),
        beklenen: "gecerli bir ISO haftasi (1..=53, o yil icinde)",
    })?;
    Ok((0..7).map(|i| baslangic.gun_ekle(i)).collect())
}

/// Günlük not için varsayılan şablon.
///
/// Şablonda **boş** bir bağlantı işareti (`[[]]`) bulunmaz: öyle bir işaret
/// `check` tarafından kırık referans olarak sayılırdı ve kullanıcı ilk günden
/// itibaren temizlenmesi gereken bir hata görürdü.
pub fn gunluk_sablonu(tarih: &Tarih) -> String {
    format!(
        "# {}\n\n## Bugun\n\n- Ozet: \n- Baglantilar: \n\n## Notlar\n\n- \n",
        tarih.iso_metni()
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn t(y: i64, a: u32, g: u32) -> Tarih {
        Tarih {
            yil: y,
            ay: a,
            gun: g,
        }
    }

    // ---- temel test vektorleri (Hinnant) ----

    #[test]
    fn epoch_sifir_yirminci_yuzyildir() {
        assert_eq!(Tarih::epoch_gununden(0), t(1970, 1, 1));
    }

    #[test]
    fn bilinen_gunler_dogru_cozulur() {
        assert_eq!(Tarih::epoch_gununden(-1), t(1969, 12, 31));
        assert_eq!(Tarih::epoch_gununden(1), t(1970, 1, 2));
        assert_eq!(Tarih::epoch_gununden(59), t(1970, 3, 1));
        assert_eq!(Tarih::epoch_gununden(365), t(1971, 1, 1));
    }

    #[test]
    fn donusum_cift_yonlu_kararlidir() {
        for gun in [-100_000i64, -1, 0, 1, 19_000, 20_700, 100_000] {
            let tarih = Tarih::epoch_gununden(gun);
            assert_eq!(tarih.gun_sayisi(), gun, "gün {gun} gidiş-dönüşü bozuk");
        }
    }

    #[test]
    fn gercek_tarihler_dogru_hesaplanir() {
        // 1970-01-01 + 20_725 gun = 2026-09-29
        assert_eq!(Tarih::epoch_gununden(20_725), t(2026, 9, 29));
        assert_eq!(Tarih::epoch_gununden(19_723), t(2024, 1, 1));
        assert_eq!(Tarih::epoch_gununden(11_139), t(2000, 7, 1));
    }

    #[test]
    fn artik_yil_kurali_uygulanir() {
        assert!(t(2024, 1, 1).artik_yil_mi());
        assert!(t(2000, 1, 1).artik_yil_mi());
        assert!(!t(1900, 1, 1).artik_yil_mi());
        assert!(!t(2026, 1, 1).artik_yil_mi());
    }

    #[test]
    fn subat_gun_sayisi_artik_yilda_29_dur() {
        assert_eq!(t(2024, 2, 1).ay_gun_sayisi(), 29);
        assert_eq!(t(2026, 2, 1).ay_gun_sayisi(), 28);
    }

    #[test]
    fn gecerlilik_ay_gun_sinirlarini_denetler() {
        assert!(t(2026, 9, 29).gecerli_mi());
        assert!(!t(2026, 13, 1).gecerli_mi());
        assert!(!t(2026, 2, 30).gecerli_mi());
        assert!(!t(2026, 4, 31).gecerli_mi());
        assert!(!t(2026, 0, 1).gecerli_mi());
        assert!(!t(2026, 1, 0).gecerli_mi());
    }

    // ---- ekleme ----

    #[test]
    fn gun_ekle_ay_yil_gezisi_yapar() {
        assert_eq!(t(2026, 9, 29).gun_ekle(1), t(2026, 9, 30));
        assert_eq!(t(2026, 9, 30).gun_ekle(1), t(2026, 10, 1));
        assert_eq!(t(2026, 12, 31).gun_ekle(1), t(2027, 1, 1));
        assert_eq!(t(2026, 1, 1).gun_ekle(-1), t(2025, 12, 31));
        assert_eq!(t(2024, 2, 28).gun_ekle(1), t(2024, 2, 29));
    }

    // ---- yazim ----

    #[test]
    fn iso_metni_sifir_doldurur() {
        assert_eq!(t(2026, 9, 5).iso_metni(), "2026-09-05");
        assert_eq!(t(999, 1, 1).iso_metni(), "0999-01-01");
    }

    #[test]
    fn dosya_adi_md_ekler() {
        assert_eq!(t(2026, 9, 29).dosya_adi(), "2026-09-29.md");
    }

    #[test]
    fn ay_adi_dogru_donulur() {
        assert_eq!(t(2026, 9, 1).ay_adi(), "Eylul");
        assert_eq!(t(2026, 1, 1).ay_adi(), "Ocak");
        assert_eq!(t(2026, 13, 1).ay_adi(), "?");
    }

    #[test]
    fn gun_adi_dogru_donulur() {
        assert_eq!(t(2026, 9, 28).gun_adi(), "Pazartesi");
        assert_eq!(t(2026, 9, 29).gun_adi(), "Sali");
        assert_eq!(t(2026, 9, 27).gun_adi(), "Pazar");
    }

    #[test]
    fn uzun_yazim_ornek_gunu_verir() {
        assert_eq!(t(2026, 9, 29).uzun(), "29 Eylul 2026 Sali");
    }

    // ---- ISO hafta ----

    #[test]
    fn hafta_gunu_pazartesiden_baslar() {
        assert_eq!(t(2026, 9, 28).hafta_gunu(), 1);
        assert_eq!(t(2026, 9, 29).hafta_gunu(), 2);
        assert_eq!(t(2026, 10, 4).hafta_gunu(), 7);
    }

    #[test]
    fn iso_hafta_numarasi_known_vektorlarla_dogru() {
        // 2026-01-01 Perşembe: ISO yıl 2026, 1. hafta.
        assert_eq!(t(2026, 1, 1).iso_hafta(), 1);
        // 2027-01-01 Cuma: ISO yıl 2026'nın 53. haftası.
        assert_eq!(t(2027, 1, 1).iso_hafta(), 53);
        // 2026-09-28: 40. hafta.
        assert_eq!(t(2026, 9, 28).iso_hafta(), 40);
    }

    #[test]
    fn hafta_baslangici_pazartesidir() {
        let p = Tarih::hafta_baslangici(2026, 40).unwrap();
        assert_eq!(p, t(2026, 9, 28));
        assert_eq!(p.hafta_gunu(), 1);
        assert_eq!(p.iso_hafta(), 40);
    }

    #[test]
    fn hafta_baslangici_gecersiz_haftada_none_verir() {
        assert!(Tarih::hafta_baslangici(2026, 0).is_none());
        assert!(Tarih::hafta_baslangici(2026, 54).is_none());
    }

    #[test]
    fn kirk_dokuz_hafta_yili_elli_uc_hftaya_sahip() {
        // 2027-01-1 Cuma oldugu icin 2026'nin 53. ISO haftasi 1 Ocak 2027'yi icerir.
        assert_eq!(Tarih::hafta_baslangici(2026, 53).unwrap(), t(2026, 12, 28));
        assert_eq!(t(2026, 12, 28).iso_hafta(), 53);
    }

    #[test]
    fn hafta_gunleri_yedi_gun_dondurur() {
        let g = hafta_gunleri(2026, 40).unwrap();
        assert_eq!(g.len(), 7);
        assert_eq!(g[0], t(2026, 9, 28));
        assert_eq!(g[6], t(2026, 10, 4));
    }

    #[test]
    fn hafta_gunleri_gecersiz_haftada_hata_doner() {
        assert!(hafta_gunleri(2026, 99).is_err());
    }

    // ---- cozumleme ----

    #[test]
    fn bugun_anahtar_kelimesi_calisir() {
        let a = Tarih::cozumle("bugun").unwrap();
        let b = Tarih::bugun();
        assert_eq!(a, b);
        assert_eq!(Tarih::cozumle("BUGUN").unwrap().iso_metni().len(), 10);
    }

    #[test]
    fn iso_tarih_cozulur() {
        assert_eq!(Tarih::cozumle("2026-09-29").unwrap(), t(2026, 9, 29));
    }

    #[test]
    fn yil_ay_cozulur_gun_bire_indirilir() {
        assert_eq!(Tarih::cozumle("2026-09").unwrap(), t(2026, 9, 1));
    }

    #[test]
    fn bozuk_tarih_hata_doner() {
        for s in [
            "",
            "yarin",
            "2026-13-01",
            "2026-02-30",
            "2026/09/29",
            "26-9-29",
        ] {
            assert!(Tarih::cozumle(s).is_err(), "{s} hata vermeli");
        }
    }

    #[test]
    fn dosya_adindan_tarih_cozulur() {
        assert_eq!(Tarih::dosya_adindan("2026-09-29"), Some(t(2026, 9, 29)));
        assert_eq!(Tarih::dosya_adindan("2026-02-30"), None);
        assert_eq!(Tarih::dosya_adindan("rapor"), None);
        assert_eq!(Tarih::dosya_adindan("2026-09-29-ek"), None);
        assert_eq!(Tarih::dosya_adindan("rapor-2026-09-29"), None);
    }

    // ---- sablon ----

    #[test]
    fn gunluk_sablonu_tarihi_icerir() {
        let s = gunluk_sablonu(&t(2026, 9, 29));
        assert!(s.starts_with("# 2026-09-29"));
        assert!(s.contains("## Bugun"));
        // Bos baglanti isareti uretilmez: `[[]]` kirik referans sayilirdi.
        assert!(!s.contains("[["));
        assert!(!s.contains("]]"));
    }
}
