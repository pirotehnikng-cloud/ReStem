<div align="center">

# ReStem

**Быстрое пакетное переименование файлов и каталогов на Rust**  
**Fast Bulk File & Folder Renamer written in Rust**

[![Version](https://img.shields.io/github/v/release/pirotehnikng-cloud/ReStem?label=Version&color=2ea44f&logo=github)](CHANGELOG.md)
[![Downloads](https://img.shields.io/github/downloads/pirotehnikng-cloud/ReStem/total?label=Downloads&color=0078d4&logo=windows)](../../releases/latest)
[![Language: Rust](https://img.shields.io/badge/Rust-2021_Edition-orange?logo=rust)](https://www.rust-lang.org/)
[![GUI: egui](https://img.shields.io/badge/GUI-eframe_%2F_egui-00b4d8)](https://github.com/emilk/egui)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

<br>

[![Скачать ReStem v1.5.0](https://img.shields.io/badge/⬇_Скачать_ReStem_v1.5.0-Прямая_ссылка_(.exe)-0078d4?style=for-the-badge&logo=windows&logoColor=white)](../../releases/latest/download/restem_1.5.0.exe)

<br>

[**🇷🇺 Русская документация**](#-restem--пакетное-переименование-файлов-и-папок) &nbsp;|&nbsp; [**🇬🇧 English Documentation**](#-restem--fast-bulk-file--folder-renamer) &nbsp;|&nbsp; [**📋 История изменений (Changelog)**](CHANGELOG.md)

</div>

---

## 🇷🇺 ReStem — Пакетное переименование файлов и папок

**ReStem** — это легковесная нативная десктопная утилита для пакетного переименования файлов и каталогов, написанная на языке **Rust** с использованием графического движка `eframe` (`egui`). Программа работает из единого портативного `.exe` файла, запускается за миллисекунды, не требует установки сторонних библиотек и плавно отображает десятки тысяч объектов благодаря виртуальной таблице.

### ✨ Ключевые возможности (v1.5.0)

* **Умные теги (Smart Tags), EXIF (JPEG/HEIC/WebP/PNG/RAW), MediaInfo и Аудио:**
  * Динамическая подстановка переменных прямо в полях префикса, суффикса, вставки по позиции, замены и шаблона подпапок:
    * `{date}`, `{year}`, `{month}`, `{day}`, `{parent}`, `{size}`, `{crc32}`, `{sha256_8}` — даты файла, родительская папка, компактный размер (`15MB`) и ленивые контрольные суммы CRC32 / SHA-256.
    * **Фото EXIF (`JPEG`, `HEIC`, `WebP`, `PNG`, `TIFF`, `DNG`, `CR2`, `NEF`, `ARW`):** `{exif_date}`, `{exif_time}` (`ЧЧ-ММ-СС`), `{camera}`, `{lens}`, `{iso}`, `{aperture}` (`f2.8`), `{shutter}` (`1-500s`), `{w}x{h}`, `{w}`, `{h}`.
    * **Видео-теги MediaInfo** (встроенный портативный парсер `MP4`, `M4V`, `MOV`, `3GP`, `MKV`, `WebM`, `AVI`, `FLV` + fallback через `MediaInfo CLI` / `ffprobe` для `TS`, `M2TS`, `WMV`, `MPG`, `VOB`, `OGV`): `{res}`, `{vcodec}`, `{acodec}`, `{ach}`, `{fps}`, `{duration}`, `{bitrate}`, `{hdr}`, `{media_date}`.
    * **Аудио-теги (`MP3` ID3v1/v2, `FLAC`, `OGG`, `Opus`, `M4A`, `WAV`):** `{artist}`, `{album}`, `{title}`, `{track}`, `{audio_year}`, `{genre}`, `{audio_bitrate}`, `{sample_rate}`.
* **Парные файлы (Sidecars), раскладка по подпапкам и ручная правка:**
  * **📎 Парные файлы:** автоматическая синхронизация `.srt`, `.ass`, `.vtt`, `.xmp`, `.nfo` и др. с основным видео или фото.
  * **📂 Раскладка по подпапкам (Organize):** перемещение файлов в динамические подпапки вроде `{year}/{month}`, `{media_date}/{res}` или `{artist}/{album}` с полным откатом `Ctrl+Z`.
  * **✏ Ручная правка строк в таблице** и **защита от конфликтов** (подсветка дубликатов, запрещённых имён Windows, путей >260 символов, автосуффикс `_1`/`_2` и безопасный 2-фазный обмен `A ↔ B`).
* **Поиск дубликатов и асинхронное сканирование:**
  * **🧬 Дубликаты:** 3-этапный поиск точных копий (размер → частичный SHA-256 → полный SHA-256) с безопасным удалением в Корзину.
  * Фоновое чтение больших каталогов с индикатором прогресса и кнопкой отмены.
* **Посимвольная Diff-подсветка, цепочки замен, порядок Pipeline, JSON-пресеты и CLI:**
  * Переключатель **«🎨 Diff-буквы»**, цепочки дополнительных шагов замены (`➕ Шаг`), перестановка порядка этапов (`Pipeline`), экспорт/импорт полных пресетов в `.json`, фоновая проверка новых релизов GitHub и CLI-автоматизация (`restem <DIR> --preset <NAME> [--dry-run | --apply]`).
* **Работа с файлами, папками и вложенными подкаталогами:**
  * Гибкая фильтрация объектов: переименовывайте только файлы, только папки или всё дерево каталогов рекурсивно.
  * **Безопасный иерархический обход (Bottom-Up):** при переименовании вложенных структур программа автоматически начинает с самого глубокого уровня вложенности и движется к корню, исключая ошибки потери пути.
  * У папок с точками в названии (например, `Отчет_2026.11`) часть после точки не воспринимается как расширение файла — имя каталога обрабатывается целиком.
* **Встроенный предпросмотр изображений, видео MediaInfo, EXIF, текста и папок:**
  * **Одиночный клик** по имени объекта в таблице открывает его в карточке **«👁 Предпросмотр объекта»** на левой панели, а **двойной клик** открывает файл в системной программе Windows по умолчанию:
    * **Изображения (`jpg`, `jpeg`, `png`, `webp`, `bmp`, `ico`, `gif`, `heic`, RAW):** масштабируемая миниатюра (для поддерживаемых форматов) и расширенные поля EXIF (разрешение, дата/время, камера, объектив, ISO, диафрагма, выдержка).
    * **Видео (`mp4`, `mkv`, `mov`, `avi`, `webm`, `m4v`, `flv`, `ts`, `wmv` и др.):** асинхронное превью кадра (при наличии `ffmpeg` в `PATH`) + карточка MediaInfo (разрешение, HDR, видео-/аудиокодеки, каналы, FPS, длительность, битрейт и дата записи).
    * **Аудио (`mp3`, `flac`, `ogg`, `opus`, `m4a`, `wav`):** встроенная обложка альбома, исполнитель, название, альбом, трек, год, жанр, длительность, битрейт и частота дискретизации.
    * **Текстовые файлы и исходный код (`txt`, `md`, `json`, `csv`, `log`, `ini`, `rs`, `py` и др.):** мгновенный предпросмотр первых 4 КБ содержимого моноширинным шрифтом `Consolas` с автоматическим распознаванием бинарных файлов.
    * **Папки:** отображение количества вложенных объектов и кнопка быстрого перехода внутрь каталога.
* **Безопасное удаление в Корзину Windows:**
  * Кнопка **«🗑 В корзину (N)»** (или клавиша `Delete`) перемещает все отмеченные галочками файлы и папки в стандартную Корзину Windows после подтверждения в диалоговом окне.
* **Гибкая сортировка списка:**
  * **Умная сортировка по имени (Natural Sort: `1, 2, 10`):** распознаёт числа внутри строк (`Кадр_2.jpg` идёт перед `Кадр_10.jpg`).
  * Сортировка по **алфавиту (`А–Я`)**, **дате изменения**, **размеру файла**, **расширению/типу** и **расположению (пути)**.
  * Быстрая сортировка в один клик по заголовкам столбцов таблицы (`Тип`, `Текущее имя`, `Размер`, `Изменён`), переключение направления (`▲ По возр.` / `▼ По убыв.`) и опция фиксации **«Папки сверху»**.
* **Нативная типографика Windows, масштабирование и автосохранение:**
  * Использование системных шрифтов Windows (`Segoe UI`, `Consolas`, `Segoe UI Emoji`), переключение языка **RU / EN**, переключение между **☀ Светлой** и **🌙 Тёмной** темой и регулировка масштаба интерфейса (**`A-` / `100%` / `A+`** от 85% до 150% для мониторов FullHD / 2K / 4K).
  * Автоматическое сохранение выбранной темы, языка, масштаба и **собственных пользовательских пресетов** между запусками.
* **Интеграция в Проводник Windows, многошаговый откат и экспорт `.bat`:**
  * Кнопка добавления пункта **«Открыть в ReStem»** в контекстное меню папок Проводника Windows прямо из окна «ℹ О программе» (без прав администратора) и поддержка передачи пути к папке через аргументы командной строки.
  * Многошаговый стек отката **«↺ Откат (Undo)»** на 20 последних пакетных операций (`Ctrl+Z`) и кнопка **«💾 .bat»** для выгрузки автономного скрипта отката `restem_rollback.bat`.

---

### 🛠 Правила переименования и фильтры (Правая панель)

1. **Поиск и замена + Шпаргалка по Regex (`[?] Regex`):**
   * Замена текста с поддержкой **регулярных выражений (Regex)**, группами захвата (`$1`, `$2`) и встроенным окном готовых Regex-шаблонов в 1 клик.
2. **Регистр, двусторонняя транслитерация и санация имён:**
   * Режимы регистра: `Не менять регистр` / `все строчные` / `ВСЕ ПРОПИСНЫЕ` / `Первая буква заглавная` / `Каждое Слово С Заглавной`.
   * **Прямая (`RU → LAT`) и обратная (`LAT → RU`) транслитерация** с распознаванием составных буквосочетаний (`shch ↔ щ`, `zh ↔ ж`, `ch ↔ ч`, `sh ↔ ш`, `yo ↔ ё`, `yu ↔ ю`, `ya ↔ я`).
   * **Санация («Чистка» имён):** автоматическое удаление двойных пробелов, пробелов перед точкой и недопустимых файловых спецсимволов.
3. **Вставка, обрезка, Умные теги и переименование по списку (`📋 По списку`):**
   * Добавление префикса, суффикса, вставка текста на заданную позицию (по индексу символа от начала или с конца имени) и посимвольная обрезка краёв.
   * Пакетное переименование по готовому списку имён из `.txt` / `.csv` или буфера обмена.
4. **Автонумерация (Numbering):**
   * Добавление номера в начало или конец имени **строго в порядке текущей сортировки таблицы** с пропуском снятых с выбора строк.
   * Настройка стартового числа, шага, разделителя, ведущих нулей (`01`, `001`) и сброса счётчика в подпапках.
5. **Расширения файлов (File Extensions):**
   * Перевод расширений в `строчные (.jpg)`, `ПРОПИСНЫЕ (.JPG)`, удаление или замена текущего расширения на новое.
6. **Фильтры и исключения (Advanced Filters):**
   * Белый и чёрный списки расширений, исключение по стоп-словам, фильтр по размеру файла (в КБ), поиск по Regex в шапке и автоматическое скрытие служебных папок (`.git`, `node_modules`).
7. **Безопасность, парные файлы, раскладка и порядок этапов (Pipeline):**
   * Автосуффикс при конфликтах имён (`_1`, `_2`), синхронизация парных файлов (`.srt`, `.ass`, `.vtt`, `.xmp` и др.), раскладка по подпапкам по шаблону и настраиваемый порядок применения правил (`▲ / ▼`).

---

### ⌨️ Горячие клавиши

| Сочетание клавиш | Действие |
|---|---|
| `Ctrl + O` | Открыть диалог выбора папки |
| `F5` | Обновить список файлов и папок |
| `Ctrl + Enter` | Применить переименование |
| `Ctrl + Z` | Многошаговый откат последнего переименования (Undo) |
| `Delete` | Переместить отмеченные галочками объекты в Корзину Windows |
| `Двойной клик мышью` | Открыть файл или папку в стандартной программе Windows |

---

### 🚀 Скачивание и запуск

1. Скачайте портативный исполняемый файл по [**прямой ссылке (`restem_1.5.0.exe`)**](../../releases/latest/download/restem_1.5.0.exe) или со страницы [**Releases**](../../releases) (история версий в [**CHANGELOG.md**](CHANGELOG.md)).
2. Запустите `restem_1.5.0.exe` — установка не требуется.
3. Нажмите **«📁 Папка...»** (`Ctrl+O`), перетащите каталог мышью в окно программы или откройте папку через контекстное меню Проводника Windows.
4. Для пакетной автоматизации в скриптах доступен CLI: `restem_1.5.0.exe "C:\Media" --preset "Video" --recursive --dry-run` (добавьте `--apply` для применения).

---

### 📄 Лицензия

Программа **ReStem** является свободным программным обеспечением и распространяется под лицензией **GNU General Public License v3.0 (GPLv3)** (см. файл [LICENSE](LICENSE)).

* **Разрешено:** бесплатное использование в личных и коммерческих целях, свободное распространение копий и модификация исходного кода.
* **Главное условие (Copyleft):** любые изменённые версии программы или производные продукты, использующие код ReStem, должны распространяться с открытым исходным кодом под той же лицензией **GNU GPL v3**. Запрещается закрывать исходный код в проприетарных сборках.
* **Отказ от ответственности:** программа предоставляется по принципу «как есть» (*AS IS*), без каких-либо гарантий.

---
---

## 🇬🇧 ReStem — Fast Bulk File & Folder Renamer

**ReStem** is a lightweight, native desktop utility for batch renaming files and directories, built in **Rust** using the `eframe` (`egui`) immediate-mode GUI framework. It runs from a single portable `.exe` with zero external dependencies, launches instantaneously, and smoothly handles tens of thousands of items using virtual table rendering.

### ✨ Key Features (v1.5.0)

* **Smart Tags, Extended EXIF (JPEG/HEIC/WebP/PNG/RAW), Video MediaInfo & Audio Tags:**
  * Dynamically insert metadata variables into Prefix, Suffix, Index Insertion, Replacement, and Subfolder Organize templates:
    * `{date}`, `{year}`, `{month}`, `{day}`, `{parent}`, `{size}`, `{crc32}`, `{sha256_8}` — file timestamps, parent directory, compact size (`15MB`), and lazy CRC32 / SHA-256 checksums.
    * **Photo EXIF (`JPEG`, `HEIC`, `WebP`, `PNG`, `TIFF`, `DNG`, `CR2`, `NEF`, `ARW`):** `{exif_date}`, `{exif_time}`, `{camera}`, `{lens}`, `{iso}`, `{aperture}`, `{shutter}`, `{w}x{h}`, `{w}`, `{h}`.
    * **Video MediaInfo Tags** (built-in portable parser for `MP4`, `M4V`, `MOV`, `3GP`, `MKV`, `WebM`, `AVI`, `FLV` + `MediaInfo CLI` / `ffprobe` fallback for `TS`, `M2TS`, `WMV`, `MPG`, `VOB`, `OGV`): `{res}`, `{vcodec}`, `{acodec}`, `{ach}`, `{fps}`, `{duration}`, `{bitrate}`, `{hdr}`, `{media_date}`.
    * **Audio Tags (`MP3` ID3v1/v2, `FLAC`, `OGG`, `Opus`, `M4A`, `WAV`):** `{artist}`, `{album}`, `{title}`, `{track}`, `{audio_year}`, `{genre}`, `{audio_bitrate}`, `{sample_rate}`.
* **Sidecar Sync, Subfolder Organize & Inline Row Editing:**
  * **📎 Sync Sidecar Files:** keeps subtitles (`.srt`, `.ass`, `.vtt`) and sidecars (`.xmp`, `.nfo`) renamed and moved alongside primary media files.
  * **📂 Organize into Subfolders:** move files into template folders like `{year}/{month}`, `{media_date}/{res}`, or `{artist}/{album}` with full `Ctrl+Z` undo.
  * **✏ Inline Table Editing & Collision Guard:** edit individual filenames inline, catch duplicate names / reserved Windows names / paths >260 chars upfront, resolve duplicates automatically (`_1`, `_2`), and perform 2-phase safe cyclic swaps (`A ↔ B`).
* **Duplicate Finder & Async Background Scanning:**
  * **🧬 Duplicate Finder:** fast 3-stage scan (size → partial SHA-256 → full SHA-256) with one-click copy selection and Recycle Bin cleanup.
  * Non-blocking directory scan with live progress indicator and Cancel button.
* **Inline Diff, Chained Replaces, Custom Pipeline, JSON Presets, CLI & Update Check:**
  * Real-time **🎨 Inline Diff**, chained Find & Replace steps (`➕ Step`), reorderable **Rule Pipeline**, full `.json` preset export/import, non-blocking GitHub release check, and headless CLI automation (`restem <DIR> --preset <NAME> [--dry-run | --apply]`).
* **Files, Folders & Recursive Subdirectories:**
  * Rename files, folders, or entire directory trees recursively using **Safe Bottom-Up Traversal** so parent folder renames never invalidate child paths.
  * Folder names containing dots (e.g., `Report_2026.11`) are treated as complete names rather than split into file extensions.
* **Built-In Image, Video MediaInfo, EXIF, Text & Folder Preview:**
  * **Single-click** any item in the table to inspect it in the **👁 Item Preview** card, or **double-click** to open it in the default Windows application:
    * **Images (`jpg`, `jpeg`, `png`, `webp`, `bmp`, `ico`, `gif`, `heic`, RAW):** proportional thumbnail preview (supported formats) plus extended EXIF metadata (dimensions, date/time, camera, lens, ISO, aperture, shutter).
    * **Videos (`mp4`, `mkv`, `mov`, `avi`, `webm`, `m4v`, `flv`, `ts`, `wmv`, etc.):** async frame thumbnail (when `ffmpeg` is on `PATH`) plus MediaInfo card (resolution, HDR, video/audio codecs, channels, FPS, duration, bitrate, recorded date).
    * **Audio (`mp3`, `flac`, `ogg`, `opus`, `m4a`, `wav`):** embedded album cover art, artist, title, album, track, year, genre, duration, bitrate, and sample rate.
    * **Text & Code Files (`txt`, `md`, `json`, `csv`, `log`, `ini`, `rs`, `py`, etc.):** instant preview of the first 4 KB of text in `Consolas` monospace font with automatic binary file detection.
    * **Folders:** displays item count and a quick button to enter the directory.
* **Safe Deletion to Windows Recycle Bin:**
  * Click **🗑 Trash (N)** (or press `Delete`) to move checked files and folders to the Windows Recycle Bin after a confirmation prompt.
* **Multi-Mode Sorting:**
  * **Natural Name Sort (`1, 2, 10`)**, **Alphabetical (`A–Z`)**, **Modified Date**, **File Size**, **Extension / Type**, and **Folder Path** sorting with clickable column headers and **Folders first** pinning.
* **Native Windows Typography, UI Scaling & Persistent Presets:**
  * Crisp Windows system fonts (`Segoe UI`, `Consolas`, `Segoe UI Emoji`), instant **RU / EN** switching, **☀ Light** / **🌙 Dark** themes, and UI zoom controls (**`A-` / `100%` / `A+`** from 85% to 150%).
  * Automatically saves your theme, language, UI scale, and **custom user presets** across sessions.
* **Windows Explorer Context Menu, Multi-Step Undo & `.bat` Export:**
  * One-click **"Open in ReStem"** Windows Explorer folder context menu registration inside the **ℹ About** dialog (no admin rights required) + CLI folder path support.
  * 20-step **↺ Undo** stack (`Ctrl+Z`) and **💾 .bat** rollback script export (`restem_rollback.bat`).

---

### 🛠 Renaming Rules & Filters (Right Panel)

1. **Find & Replace + Interactive Regex Cheat Sheet (`[?] Regex`):**
   * Plain text or full **Regular Expression (Regex)** matching with capture group support (`$1`, `$2`) and built-in one-click Regex templates.
2. **Case, Bidirectional Transliteration & Name Sanitization:**
   * Case modes: `Default (No change)` / `lowercase` / `UPPERCASE` / `Capitalize first letter` / `Title Case Words`.
   * **Bidirectional Transliteration (`RU → LAT` & `LAT → RU`):** converts Cyrillic to Latin and vice versa (`shch ↔ щ`, `zh ↔ ж`, `ch ↔ ч`, `sh ↔ ш`, `yo ↔ ё`, `yu ↔ ю`, `ya ↔ я`).
   * **Sanitize Names:** collapses multiple spaces, strips trailing spaces before extensions, and removes illegal filesystem characters.
3. **Add, Trim, Smart Tags & Rename from List (`📋 From List`):**
   * Prepend/append text, insert text at a specific character index (from start or end), trim characters, or batch rename directly from a `.txt` / `.csv` list or clipboard.
4. **Numbering (Follows Sort Order):**
   * Sequential numbering strictly following the current table sort order with customizable start, step, separator, zero-padding (`01`, `001`), and per-folder reset.
5. **File Extensions:**
   * Transform extensions to `lowercase (.jpg)`, `UPPERCASE (.JPG)`, remove them, or replace them with a custom extension.
6. **Advanced Filters:**
   * Extension whitelist/blacklist, comma-separated word exclusion, file size range filter (in KB), regex toolbar search, and automatic hiding of `.git` and system folders.
7. **Safety, Sidecars, Organize & Pipeline Order:**
   * Duplicate collision auto-suffixing (`_1`, `_2`), automatic sidecar syncing (`.srt`, `.ass`, `.vtt`, `.xmp`, etc.), template subfolder organization, and reorderable rule stages (`▲ / ▼`).

---

### ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl + O` | Open folder selection dialog |
| `F5` | Refresh current directory |
| `Ctrl + Enter` | Apply rename changes |
| `Ctrl + Z` | Multi-step Undo (up to 20 batch operations) |
| `Delete` | Move checked items to Windows Recycle Bin |
| `Double-Click` | Open file or folder in default Windows application |

---

### 🚀 Download & Usage

1. Download [**`restem_1.5.0.exe` (Direct Download)**](../../releases/latest/download/restem_1.5.0.exe) or visit the [**Releases**](../../releases) page (see [**CHANGELOG.md**](CHANGELOG.md) for version history).
2. Run `restem_1.5.0.exe` — no installation or runtime required.
3. Click **📁 Folder...** (`Ctrl+O`), drag and drop a folder into the window, or open any folder via the Windows Explorer context menu.
4. For headless script automation, use CLI flags: `restem_1.5.0.exe "C:\Media" --preset "Video" --recursive --dry-run` (add `--apply` to commit renames).

---

### 📄 License

**ReStem** is free and open-source software distributed under the terms of the **GNU General Public License v3.0 (GPLv3)** (see [LICENSE](LICENSE) for full details).

* You are free to use, study, share, and modify the software for personal or commercial purposes.
* Any derivative works or modified distributions must remain open-source and be licensed under the same **GNU GPL v3** terms.
