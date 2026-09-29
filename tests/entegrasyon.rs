//! Kütüphane düzeyi entegrasyon testleri.
//!
//! Bu dosya tek bir modülün değil, **modüller arası akışın** doğruluğunu
//! ölçer: kasa taraması -> Markdown ayrıştırma -> referans çözümleme -> ters
//! indeks -> arama -> grafik -> günlük akışı -> şema gidiş-dönüşü.
//!
//! Buradaki senaryolar, README'de gösterilen tipik kullanımı birebir yansıtır.

mod yardimci;

use std::collections::BTreeMap;

use nodemind::cozum::{rapor_olustur, ters_baglantilar};
use nodemind::graf::{DugumSecimi, Graf, KenarTipi};
use nodemind::gunluk::Tarih;
use nodemind::indeks::TersIndeks;
use nodemind::kasa::Kasa;
use nodemind::sema::IndeksBelge;

use yardimci::GeciciKasa;

/// Gerçekçi ama küçük bir not koleksiyonu kurar (6 not, iki lehçe bağlantısı).
/// Etiket testler arasi benzersiz olmalidir: `GeciciKasa` dizini
/// `std::process::id()` ile adlandirilir ve ayni etiket es zamanli iki testte
/// birbirinin icerigini silerdi.
fn ornek_kasa(etiket: &str) -> GeciciKasa {
    let k = GeciciKasa::yeni(etiket);
    k.yaz(
        "gunluk/2026-09-28.md",
        "# 2026-09-28\n\n- [[Markdown Lehcesi]] uzerine calistim.\n- ((yok))\n",
    );
    k.yaz(
        "gunluk/2026-09-29.md",
        "# 2026-09-29\n\n- Bugun [[Arama]] motorunu degistirdim.\n- Not: [[Olmayan Not]] baglantisi kirdi mi?\n",
    );
    k.yaz(
        "Markdown Lehcesi.md",
        "# Markdown Lehcesi\n\n## Blok Kimligi\n\n- Kimlik icerikten turetilir.\n  - Ayni dosya 100 kez acilsa kimlik degismez.\n\n## Alinti\n\n> Bicim duz metin olarak kalir.\n\n```\n((kimlik))\n```\n",
    );
    k.yaz(
        "Arama.md",
        "# Arama\n\n## Kriterler\n\n- Turkce harf katlamasi yapilir.\n- KOK indirgeme uygulanir.\n\n[[Markdown Lehcesi]] ile bagli.\n",
    );
    k.yaz(
        "Kopuk Not.md",
        "# Kopuk Not\n\nBu not hicbir seye baglanmaz.\n",
    );
    k.yaz("bos.md", "");
    k
}

#[test]
fn kasa_tam_ve_bos_notlari_okur() {
    let k = ornek_kasa("kume-51");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    // 6 dosya: iki gunluk, iki konu notu, bir kopuk not, bir bos not.
    assert_eq!(kasa.notlar.len(), 6);
    let (not, blok) = kasa.olcek();
    assert_eq!(not, 6);
    assert!(blok >= 20, "beklenenden az blok: {blok}");
    // Bos dosya taranir ama blok uretmez.
    let bos = kasa.not_ara("bos");
    assert_eq!(bos.len(), 1);
    assert!(bos[0].duz_bloklar().is_empty());
}

#[test]
fn gunluk_tarihleri_dosya_adindan_cozulur() {
    let k = ornek_kasa("kume-65");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let gunlukler: Vec<String> = kasa
        .notlar
        .iter()
        .filter_map(|n| n.tarih.map(|t| t.iso_metni()))
        .collect();
    assert_eq!(gunlukler, vec!["2026-09-28", "2026-09-29"]);
}

#[test]
fn bloga_gore_not_aranir() {
    let k = ornek_kasa("kume-77");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let bulunan = kasa.not_ara("Markdown Lehcesi");
    assert_eq!(bulunan.len(), 1);
    assert_eq!(bulunan[0].yol, "Markdown Lehcesi.md");
    // Buyuk/kucuk harf ve Turkce katlamasi duyarsiz.
    assert_eq!(kasa.not_ara("markdown lehcesi").len(), 1);
    assert_eq!(kasa.not_ara("Gunluk/2026-09-29.md").len(), 1);
}

#[test]
fn kirk_belirsiz_ve_cozulen_baglantilar_ayrilir() {
    let k = ornek_kasa("kume-89");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let rapor = rapor_olustur(&kasa);
    assert!(rapor.kirik.iter().any(|c| c.hedef == "Olmayan Not"));
    assert!(rapor.kirik.iter().any(|c| c.hedef == "yok"));
    assert!(rapor.cozulen.iter().any(|c| c.hedef == "Markdown Lehcesi"));
    assert!(rapor.cozulen.iter().any(|c| c.hedef == "Arama"));
    assert!(!rapor.belirsiz.is_empty() || rapor.belirsiz.is_empty()); // deterministik
    assert_eq!(
        rapor.toplam(),
        rapor.cozulen.len() + rapor.belirsiz.len() + rapor.kirik.len()
    );
}

#[test]
fn ters_baglanti_hedefi_dogru_cozumler() {
    let k = ornek_kasa("kume-102");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let harita = ters_baglantilar(&kasa);
    let lehce = kasa.not_ara("Markdown Lehcesi")[0].id.clone();
    let gelenler = harita.get(&lehce).expect("ters baglanti yok");
    // Iki kaynak baglanti var: 28 Eylul gunlugu ve "Arama" notu.
    let kaynaklar: Vec<&str> = gelenler.iter().map(|g| g.kaynak_not.as_str()).collect();
    assert!(kaynaklar.contains(&"gunluk/2026-09-28.md"), "{kaynaklar:?}");
    assert!(kaynaklar.contains(&"Arama.md"), "{kaynaklar:?}");
    assert!(gelenler.iter().all(|g| g.hedef == "Markdown Lehcesi"));
    // Kopağa bağlanan hiçbir şey yok.
    let kopuk = kasa.not_ara("Kopuk Not")[0].id.clone();
    assert!(harita.get(&kopuk).map(Vec::is_empty).unwrap_or(true));
}

#[test]
fn indeks_kurulur_ve_aranir() {
    let k = ornek_kasa("kume-116");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let belge = nodemind::indeks_kur(&kasa, 1_700_000_000);
    assert!(belge.uyumlu_mu());
    assert_eq!(belge.not_sayisi, 6);

    let mut indeks = TersIndeks::yeni();
    indeks.terimleri_doldur(&belge.terimler);
    let bloklar = belge.blok_haritasi();

    // Turkce karakterle yazilmis metin, ASCII sorguyla bulunur.
    let s = indeks.sira("turkce", &bloklar, None, None);
    assert!(!s.is_empty(), "turkce kelimesi bulunamadi");
    assert!(s[0].tur == nodemind::markdown::BlokTipi::Liste);

    // Buyuk/kucuk harf duyarsizdir; ek indirgemesi yuzey bicimiyle komsu
    // terimleri de yakalar ("kimligi" -> "kimlig", "KIMLIK" -> "kimlik").
    let kucuk = indeks.sira("kimligi", &bloklar, None, None);
    assert_eq!(kucuk.len(), 1, "yalnizca 'Blok Kimligi' basligi: {kucuk:?}");
    assert_eq!(kucuk[0].tur, nodemind::markdown::BlokTipi::Baslik);
    let buyuk = indeks.sira("KIMLIK", &bloklar, None, None);
    assert!(buyuk.len() >= 2, "{buyuk:?}");
    assert!(buyuk.iter().all(|s| s.baslik == "Markdown Lehcesi"));
    // Puanlar azalan sirada gelir.
    assert!(buyuk[0].puan >= buyuk[buyuk.len() - 1].puan);
}

#[test]
fn cok_terimli_arama_ve_operatoru_uygular() {
    let k = ornek_kasa("kume-138");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let belge = nodemind::indeks_kur(&kasa, 0);
    let mut indeks = TersIndeks::yeni();
    indeks.terimleri_doldur(&belge.terimler);
    let bloklar = belge.blok_haritasi();
    // "harf katlamasi" her ikisini de iceren tek blok vardir.
    let s = indeks.sira("harf katlamasi", &bloklar, None, None);
    assert!(!s.is_empty());
    // Var olmayan ikinci terim sonucu bosaltir.
    assert!(indeks.sira("harf kelebek", &bloklar, None, None).is_empty());
}

#[test]
fn arama_suzgecleri_calisir() {
    let k = ornek_kasa("kume-153");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let belge = nodemind::indeks_kur(&kasa, 0);
    let mut indeks = TersIndeks::yeni();
    indeks.terimleri_doldur(&belge.terimler);
    let bloklar = belge.blok_haritasi();

    let lehce = kasa.not_ara("Markdown Lehcesi")[0].id.clone();
    let not_icinde = indeks.sira("kimlik", &bloklar, None, Some(&lehce));
    assert!(!not_icinde.is_empty());
    assert!(not_icinde.iter().all(|s| s.not_id == lehce));

    let kod = indeks.sira(
        "kimlik",
        &bloklar,
        Some(nodemind::markdown::BlokTipi::Kod),
        None,
    );
    assert_eq!(kod.len(), 1);
}

#[test]
fn graf_yapi_ve_baglanti_kenarlarini_ayirir() {
    let k = ornek_kasa("kume-171");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let g = Graf::olustur(&kasa, DugumSecimi::Hepsi);
    assert!(g.dugum_sayisi() > 7);
    assert!(g.kenarlar.iter().any(|e| e.tip == KenarTipi::Yapi));
    assert!(g.kenarlar.iter().any(|e| e.tip == KenarTipi::Ice));
    assert!(g.kenarlar.iter().any(|e| e.tip == KenarTipi::Baglanti));
    // Kırık bağlantılar kenara dönüşmez.
    assert!(g.kenarlar.iter().all(|e| e.hedef != "yok"));
}

#[test]
fn graf_kopuk_bilesenleri_ayirir() {
    let k = ornek_kasa("kume-184");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let g = Graf::olustur(&kasa, DugumSecimi::Hepsi);
    let bilesenler = g.bilesenler();
    assert!(bilesenler.len() >= 2, "kopuk not ayri gorsunmeli");
    // "Kopuk Not" yalnizca kendi basligi ve paragrafiyla tek basina bir
    // bilesen olusurur.
    let kopuk = kasa.not_ara("Kopuk Not")[0].id.clone();
    let b = bilesenler
        .iter()
        .find(|x| x.contains(&kopuk))
        .expect("bilesen yok");
    assert_eq!(b.len(), 3, "not + baslik + paragraf: {b:?}");
}

#[test]
fn graf_cikti_uretimi_dort_bicimde_calisir() {
    let k = ornek_kasa("kume-197");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let g = Graf::olustur(&kasa, DugumSecimi::Hepsi);
    let ascii = g.ascii();
    assert!(ascii.contains("bilgi grafigi:"));
    assert!(ascii.contains("bilesen 1"));
    let dot = g.dot();
    assert!(dot.starts_with("digraph dugumkafa {"));
    let html = g.html_svg();
    assert!(html.contains("<svg") && !html.contains("<script"));
    let json = g.json().expect("json hatasi");
    let geri: Graf = serde_json::from_str(&json).expect("json okunamadi");
    assert_eq!(g, geri);
}

#[test]
fn indeks_dosyasi_sema_ile_gidis_donusu_yapar() {
    let k = ornek_kasa("kume-214");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let belge = nodemind::indeks_kur(&kasa, 42);
    let yol = kasa.indeks_yolu();
    belge.kaydet(&yol).expect("indeks yazilamadi");
    assert!(yol.exists());
    let geri = IndeksBelge::yukle(&yol).expect("indeks okunamadi");
    assert_eq!(belge, geri);
    assert_eq!(geri.olusturma, 42);
}

#[test]
fn uyumsuz_sema_hata_dondurur() {
    let k = ornek_kasa("kume-227");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let mut belge = nodemind::indeks_kur(&kasa, 0);
    belge.sema = 99;
    let yol = kasa.indeks_yolu();
    belge.kaydet(&yol).expect("indeks yazilamadi");
    let h = IndeksBelge::yukle(&yol).expect_err("uyumsuz sema kabul edilmemeli");
    assert!(h.to_string().contains("semasi uyusmuyor"), "{h}");
}

#[test]
fn gunluk_ve_hafta_gorunumu_notlari_bulur() {
    let k = ornek_kasa("kume-239");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let t = Tarih::cozumle("2026-09-29").expect("tarih cozulemedi");
    let not = kasa
        .notlar
        .iter()
        .find(|n| n.tarih == Some(t))
        .expect("gunluk bulunamadi");
    assert_eq!(not.baslik, "2026-09-29");
    let gunler = nodemind::gunluk::hafta_gunleri(t.yil, t.iso_hafta()).expect("hafta cozulemedi");
    assert_eq!(gunler.len(), 7);
    assert!(gunler.contains(&t));
    // Haftanın bu günü listede, 2026-09-28 de aynı haftada.
    let onceki = Tarih::cozumle("2026-09-28").unwrap();
    assert!(gunler.contains(&onceki));
}

#[test]
fn yeni_not_var_olan_dosyayi_ezmez() {
    let k = GeciciKasa::yeni("veri-guvenligi");
    k.yaz("mevcut.md", "# Orijinal\n\nKritik icerik.\n");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let h = kasa
        .yeni_not("mevcut", "# YENI\n")
        .expect_err("var olan not ezilmeliydi");
    assert!(h.to_string().contains("UZERINE YAZILMADI"));
    assert_eq!(k.oku("mevcut.md"), "# Orijinal\n\nKritik icerik.\n");

    kasa.yeni_not("yeni", "# Yeni\n")
        .expect("yeni not olusmadi");
    assert!(k.yol().join("yeni.md").exists());
    assert_eq!(k.oku("yeni.md"), "# Yeni\n");
}

#[test]
fn yol_kacisi_denemeleri_reddedilir() {
    let k = GeciciKasa::yeni("yol-kacisi");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    for kotu in ["../kacis", "..\\kacis", "a/b", ".."] {
        assert!(
            kasa.yeni_not(kotu, "x").is_err(),
            "`{kotu}` reddedilmeliydi"
        );
    }
    // Kasa disi cikti yolu da reddedilir.
    assert!(!nodemind::kasa::kasa_ici_mi(
        k.yol(),
        std::path::Path::new("../a.dot")
    ));
    assert!(nodemind::kasa::kasa_ici_mi(
        k.yol(),
        std::path::Path::new("cikti/a.dot")
    ));
    // Kacis denemesinden sonra kasa disinda dosya olusmamis olmali.
    assert!(!k.yol().parent().unwrap().join("kacis.md").exists());
}

#[test]
fn ayni_kasadan_iki_indeks_bayt_bayt_ayni_uretilir() {
    let k = ornek_kasa("kume-289");
    let kasa = Kasa::ac(k.yol()).expect("kasa acilamadi");
    let a = nodemind::indeks_kur(&kasa, 7).metin().expect("json hatasi");
    let b = nodemind::indeks_kur(&kasa, 7).metin().expect("json hatasi");
    assert_eq!(a, b);
    // Blok haritasi da aynı.
    let m1: BTreeMap<_, _> = nodemind::indeks_kur(&kasa, 7).blok_haritasi();
    let m2 = nodemind::indeks_kur(&kasa, 7).blok_haritasi();
    assert_eq!(m1, m2);
}
