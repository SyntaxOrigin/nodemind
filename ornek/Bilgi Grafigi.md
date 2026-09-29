# Bilgi Grafigi

Baglantilardan turen yonlu graf ve uc ayri gosterim.

## Kenar Turleri

- **Yapi**: not -> kok blogu.
- **Ic**: ebeveyn blok -> cocuk blok.
- **Baglanti**: kaynak blok -> hedef not veya hedef blok.

## Gosterimler

- Terminal: ASCII agac (`+--`, `|`, `--`), her yonlu bilesen ayri numaralandirilir.
  - Ayni yonlulukte iki kenar varsa **doner kenar** bilesen altinda `cevrim:` satirinda listelenir.
  - Boylece donguler sessizce kaybolmaz.
- Disa aktarim: JSON, Graphviz DOT, bagimsiz HTML/SVG (JavaScript yok, dis kaynak yok).

## Kirik Baglantilar

Kirik ve belirsiz baglantilar **kenara donusmez**; `nodemind check` bunlari
ayri listeler. [[Olmayan Not]] henuz yazilmadi, bu bir kirk baglantidir.

[[Arama]] ile baglantilidir.