//! Komut satırı entegrasyon testleri.
//!
//! Bu dosya **derlenmiş ikiliyi** çalıştırır: argüman ayrıştırma, çıktı
//! biçimlendirme, çıkış kodları ve veri güvenliği davranışı burada ölçülür.
//!
//! İkili `cargo test` sırasında `CARGO_BIN_EXE_nodemind` ile mutlak yolda
//! verilir; `target/debug` yolunu tahmin etmeye gerek bırakmaz.

mod yardimci;

use yardimci::{calistir, GeciciKasa};

/// Etiket testler arasi benzersiz olmalidir: GeciciKasa dizini
/// std::process::id() ile adlandirir ve ayni etiket es zamanli iki testte
/// birbirinin icerigini silerdi.
fn ornek(etiket: &str) -> GeciciKasa {
    let k = GeciciKasa::yeni(etiket);
    k.yaz(
        "gunluk/2026-09-29.md",
        "# 2026-09-29\n\n- [[Arama]] motorunu yeniledim.\n- [[Olmayan]] baglanti.\n",
    );
    k.yaz(
        "Arama.md",
        "# Arama\n\n## Kriterler\n\n- Turkce harf katlamasi.\n- KOK indirgeme.\n",
    );
    k.yaz("Kopuk.md", "# Kopuk\n\nTek basina duruyor.\n");
    k
}

#[test]
fn yardim_ciktisi_basarili_kodonla_biter() {
    let k = ornek("cli-32");
    let c = calistir(&k, &["check"]);
    assert_eq!(c.kod, 2, "kırık bağlantı varken `check` 2 dönmeli: {c:?}");
    c.icermeli("Olmayan");
    c.icermeli("SORUNLU");
}

#[test]
fn temiz_kasa_check_sifir_doner() {
    let k = GeciciKasa::yeni("cli-temiz");
    k.yaz("a.md", "# A\n");
    let c = calistir(&k, &["check"]);
    assert_eq!(c.kod, 0, "{c:?}");
    c.icermeli("TEMIZ");
}

#[test]
fn index_indeks_dosyasini_yazar() {
    let k = ornek("cli-50");
    let c = calistir(&k, &["index"]);
    assert_eq!(c.kod, 0, "{c:?}");
    c.icermeli("cache");
    assert!(k.yol().join("cache").join("index.json").exists());
    c.icermeli("sema surumu: 1");
}

#[test]
fn search_indeks_kullanir_ve_turkce_sorguyu_bulur() {
    let k = ornek("cli-60");
    calistir(&k, &["index"]);
    let c = calistir(&k, &["search", "turkce"]);
    assert_eq!(c.kod, 0, "{c:?}");
    c.icermeli("eslesme");
    c.icermeli("Arama");
}

#[test]
fn search_eslesme_yoksa_acikca_soyler() {
    let k = ornek("cli-70");
    let c = calistir(&k, &["search", "boyle-bir-kavram-yok"]);
    assert_eq!(c.kod, 0, "{c:?}");
    c.icermeli("eslesme yok");
}

#[test]
fn search_tur_ve_not_suzgeci_calisir() {
    let k = ornek("cli-78");
    let c = calistir(&k, &["search", "kriterler", "--tur", "baslik"]);
    c.icermeli("[baslik]");
    let c2 = calistir(&k, &["search", "indirgeme", "--not", "Arama"]);
    c2.icermeli("Arama");
    c2.icermemeli("Kopuk");
}

#[test]
fn backlinks_hedefe_baglananlari_listeler() {
    let k = ornek("cli-88");
    let c = calistir(&k, &["backlinks", "Arama"]);
    assert_eq!(c.kod, 0, "{c:?}");
    c.icermeli("gunluk/2026-09-29.md");
    c.icermeli("ters baglanti");
}

#[test]
fn backlinks_bulunamayan_hedefte_uyari_verir() {
    let k = ornek("cli-97");
    let c = calistir(&k, &["backlinks", "YokBoyleNot"]);
    c.icermeli("bulunamadi");
}

#[test]
fn graph_ascii_ciktisi_uretir() {
    let k = ornek("cli-104");
    let c = calistir(&k, &["graph"]);
    assert_eq!(c.kod, 0, "{c:?}");
    c.icermeli("bilgi grafigi:");
    c.icermeli("bilesen 1");
    c.icermeli("en cok baglantilanan notlar");
}

#[test]
fn graph_dort_bicimi_dosyaya_yazar() {
    let k = ornek("cli-114");
    for (bicim, dosya, beklenen) in [
        ("json", "cikti/g.json", "\"dugumler\""),
        ("dot", "cikti/g.dot", "digraph dugumkafa"),
        ("html", "cikti/g.html", "<svg"),
        ("ascii", "cikti/g.txt", "bilgi grafigi:"),
    ] {
        let c = calistir(&k, &["graph", "--bicim", bicim, "--cikti", dosya]);
        assert_eq!(c.kod, 0, "{bicim}: {c:?}");
        let icerik = k.oku(dosya);
        assert!(
            icerik.contains(beklenen),
            "{bicim} ciktisi beklenmeyen: {beklenen}"
        );
    }
}

#[test]
fn graph_kasa_disi_cikti_yolunu_reddeder() {
    let k = ornek("cli-130");
    let c = calistir(&k, &["graph", "--cikti", "../kacis.dot"]);
    assert_eq!(c.kod, 1, "{c:?}");
    c.icermeli("kasa disi yol");
}

#[test]
fn today_bugun_gucunu_gosterir_ve_olusurmaz() {
    // Bos kasa: bugunun gunluku **yoktur** ve komut dosya yaratmaz.
    // (Test duvar saatine bagli olmamak icin ornek koleksiyonu kullanilmaz.)
    let k = GeciciKasa::yeni("cli-today-bugun");
    let c = calistir(&k, &["today", "--bugun"]);
    assert_eq!(c.kod, 0, "{c:?}");
    c.icermeli("gunluk notu yok");
    c.icermeli("--yaz");
    let yazilan: Vec<String> = std::fs::read_dir(k.yol())
        .expect("kasa okunamadi")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(yazilan.is_empty(), "olmayan dosya yazildi: {yazilan:?}");
}

#[test]
fn today_var_olan_gunlugu_bloklariyla_gosterir() {
    let k = ornek("cli-today-mevcut");
    let c = calistir(&k, &["today", "--tarih", "2026-09-29"]);
    assert_eq!(c.kod, 0, "{c:?}");
    c.icermeli("dosya: gunluk/2026-09-29.md");
    c.icermeli("[madde]");
}

#[test]
fn today_yaz_ile_yeni_gunluk_olusturur() {
    let k = GeciciKasa::yeni("cli-today-yaz");
    let c = calistir(&k, &["today", "--bugun", "--yaz"]);
    assert_eq!(c.kod, 0, "{c:?}");
    c.icermeli("olusturuldu");
    let ad = format!("{}.md", nodemind::gunluk::Tarih::bugun().iso_metni());
    let ilk = k.oku(&ad);

    // Ikinci calistirma **idempotenttir**: gunluk zaten varsa gosterilir ve
    // dosya bayt bayt degistirilmez (veri guvenligi kurali).
    let c2 = calistir(&k, &["today", "--bugun", "--yaz"]);
    assert_eq!(c2.kod, 0, "{c2:?}");
    c2.icermeli("dosya: ");
    c2.icermemeli("olusturuldu:");
    assert_eq!(k.oku(&ad), ilk, "gunluk dosyasi degistirilmemeliydi");
}

#[test]
fn new_var_olan_notu_ustune_yazmaz() {
    let k = GeciciKasa::yeni("cli-new-ustune");
    k.yaz("var.md", "# Orijinal\n\nKritik icerik.\n");
    let c = calistir(&k, &["new", "var"]);
    assert_eq!(c.kod, 1, "{c:?}");
    c.icermeli("UZERINE YAZILMADI");
    assert_eq!(k.oku("var.md"), "# Orijinal\n\nKritik icerik.\n");
}

#[test]
fn today_bugun_ve_tarih_birlikte_verilemez() {
    let k = GeciciKasa::yeni("cli-today-cakisma");
    let c = calistir(&k, &["today", "--bugun", "--tarih", "2026-09-29"]);
    assert_eq!(c.kod, 1, "{c:?}");
    c.icermeli("gecersiz secenek kombinasyonu");
}

#[test]
fn today_tarih_ve_hafta_secenekleri_calisir() {
    let k = ornek("cli-160");
    let c = calistir(&k, &["today", "--tarih", "2026-09-29"]);
    assert_eq!(c.kod, 0, "{c:?}");
    c.icermeli("2026-09-29");
    let h = calistir(
        &k,
        &["today", "--tarih", "2026-09-29", "--gorunum", "hafta"],
    );
    h.icermeli("ISO haftasi 40");
    h.icermeli("2026-09-28");
    h.icermeli("(gunluk yok)");
}

#[test]
fn today_bozuk_tarihte_hata_verir() {
    let k = ornek("cli-171");
    let c = calistir(&k, &["today", "--tarih", "2026-13-45"]);
    assert_eq!(c.kod, 1, "{c:?}");
    c.icermeli("tarih cozulemedi");
}

#[test]
fn new_not_olusturur_ve_var_olani_ezmez() {
    let k = GeciciKasa::yeni("cli-new");
    let c = calistir(&k, &["new", "Yeni Not", "--baslik", "Yeni Notun Basligi"]);
    assert_eq!(c.kod, 0, "{c:?}");
    c.icermeli("olusturuldu");
    assert!(k.oku("Yeni Not.md").contains("# Yeni Notun Basligi"));

    let c2 = calistir(&k, &["new", "Yeni Not"]);
    assert_eq!(c2.kod, 1, "{c2:?}");
    c2.icermeli("UZERINE YAZILMADI");
    assert!(k.oku("Yeni Not.md").contains("# Yeni Notun Basligi"));
}

#[test]
fn new_yol_kacisi_adini_reddeder() {
    let k = GeciciKasa::yeni("cli-new-kacis");
    let c = calistir(&k, &["new", "../kacis"]);
    assert_eq!(c.kod, 1, "{c:?}");
    c.icermeli("gecersiz not adi");
    assert!(!k.yol().parent().unwrap().join("kacis.md").exists());
}

#[test]
fn check_json_raporu_machine_okunur_uretir() {
    let k = ornek("cli-202");
    let c = calistir(&k, &["check", "--json"]);
    assert_eq!(c.kod, 2, "{c:?}");
    c.icermeli("\"sema\"");
    c.icermeli("\"kirik\"");
    c.icermeli("\"belirsiz\"");
}

#[test]
fn check_sema_uyumsuzlugunu_bildirir() {
    let k = ornek("cli-212");
    calistir(&k, &["index"]);
    let yol = k.yol().join("cache").join("index.json");
    let mut belge: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&yol).unwrap()).unwrap();
    belge["sema"] = serde_json::json!(42);
    std::fs::write(&yol, serde_json::to_string_pretty(&belge).unwrap()).unwrap();
    let c = calistir(&k, &["check"]);
    assert_eq!(c.kod, 2, "{c:?}");
    c.icermeli("UYUMSUZ");
}

#[test]
fn olmayan_kasa_hata_donderir() {
    let k = GeciciKasa::yeni("cli-yok");
    // Var olmayan bir alt dizin kasa olarak verilir.
    let c = std::process::Command::new(env!("CARGO_BIN_EXE_nodemind"))
        .arg("--kasa")
        .arg(k.yol().join("olmayan"))
        .arg("graph")
        .output()
        .expect("ikili calismadi");
    let stderr = String::from_utf8_lossy(&c.stderr).to_string();
    assert_eq!(c.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("bu yol bir klasor degil"), "{stderr}");
}

#[test]
fn yardim_ve_surum_bilgisi_uretilir() {
    let k = ornek("cli-234");
    let c = calistir(&k, &["--version"]);
    assert_eq!(c.kod, 0, "{c:?}");
    c.icermeli("0.1.0");
}
