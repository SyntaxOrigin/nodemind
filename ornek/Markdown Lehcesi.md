# Markdown Lehcesi

NodeMind'in destekledigi Markdown alt kumesi ve blok kimligi kurallari.

## Blok Kimligi

- Kimlik **icerikten** turetilir: not kimligi, konum yolu, blok tipi ve metin birlikte FNV-1a 64 ozetine girer.
  - Ayni dosya 100 kez acilsa kimlikler degismez.
  - Bir blogun metni degisirse kimligi degisir ve o bloga yazilmis baglantilar kirk isaretlenir.
- Kimlik dosyaya **yazilmaz**. Notlar duz Markdown olarak kalir.
- Kimlik onaltilik ve 12 karakter uzunlugundadir; ayni kimlik iki kez uretilirse -2, -3 eklenir.

## Blok Turleri

- Baslik (# ... ######)
- Liste maddesi (-, *, +, 12., 12))
- Alinti (>)
- Kod blogu (`` ` ``)
- Paragraf

## Alinti

> Bicim duz metin olarak kalir; ayristirici degisse bile notlar okunabilir kalir.

Alinti isaretcileri soyulur ve alt metin yeniden ayristirilir. Bu yuzden asagidaki
satir bir listenin cocugudur:

> - Alinti icindeki madde
> - Alinti icindeki ikinci madde

## Desteklenmeyenler

Tablolar, satir kirlamalari, satir ici bicimlendirme ve HTML blogu ayristirilmaz;
metin oldugu gibi paragraf olarak saklanir.