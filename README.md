<div align="center">

# ReStem

**Быстрое пакетное переименование файлов и каталогов на Rust**  
**Fast Bulk File & Folder Renamer written in Rust**

[![Version](https://img.shields.io/github/v/release/pirotehnikng-cloud/ReStem?label=Version&color=2ea44f&logo=github)](CHANGELOG.md)
[![Downloads](https://img.shields.io/github/downloads/pirotehnikng-cloud/ReStem/total?label=Downloads&color=0078d4&logo=windows)](../../releases/latest)
[![Language: Rust](https://img.shields.io/badge/Rust-2021_Edition-orange?logo=rust)](https://www.rust-lang.org/)
[![GUI: egui](https://img.shields.io/badge/GUI-eframe_%2F_egui-00b4d8)](https://github.com/emilk/egui)
[![License: MIT](https://img.shields.io/badge/License-MIT-success)](LICENSE)

<br>

[![Скачать ReStem v1.3.0](https://img.shields.io/badge/⬇_Скачать_ReStem_v1.3.0-Прямая_ссылка_(.exe)-0078d4?style=for-the-badge&logo=windows&logoColor=white)](../../releases/latest/download/restem_1.3.0.exe)

<br>

[**🇷🇺 Русская документация**](#-restem--пакетное-переименование-файлов-и-папок) &nbsp;|&nbsp; [**🇬🇧 English Documentation**](#-restem--fast-bulk-file--folder-renamer) &nbsp;|&nbsp; [**📋 История изменений (Changelog)**](CHANGELOG.md)

</div>

---

## 🇷🇺 ReStem — Пакетное переименование файлов и папок

**ReStem** — это легковесная нативная десктопная утилита для пакетного переименования файлов и каталогов, написанная на языке **Rust** с использованием графического движка `eframe` (`egui`). Программа работает из единого портативного `.exe` файла, запускается за миллисекунды, не требует установки сторонних библиотек и плавно отображает десятки тысяч объектов благодаря виртуальной таблице.

### ✨ Ключевые возможности (v1.3.0)

* **Умные теги (Smart Tags) и чтение метаданных EXIF:**
  * Динамическая подстановка переменных прямо в полях префикса, суффикса, вставки по позиции и замены:
    * `{date}`, `{year}`, `{month}`, `{day}` — дата изменения файла (`ГГГГ-ММ-ДД`).
    * `{exif_date}` — реальная дата съёмки фотографии из EXIF-заголовка JPEG.
    * `{camera}` — модель фотокамеры или смартфона из EXIF (например, `Canon_EOS_R6`).
    * `{parent}` — имя родительской папки файла.
    * `{w}x{h}`, `{w}`, `{h}` — точное разрешение изображения в пикселях (например, `1920x1080`), считываемое из заголовка файла без декодирования всей картинки.
* **Посимвольная Diff-подсветка в реальном времени:**
  * Переключатель **«🎨 Diff-подсветка букв»** на верхней панели наглядно выделяет красным зачёркнутым шрифтом удаляемые фрагменты в текущем имени и ярко-зелёным — добавленные или изменённые символы в новом имени.
* **Работа с файлами, папками и вложенными подкаталогами:**
  * Гибкая фильтрация объектов: переименовывайте только файлы, только папки или всё дерево каталогов рекурсивно.
  * **Безопасный иерархический обход (Bottom-Up):** при переименовании вложенных структур программа автоматически начинает с самого глубокого уровня вложенности и движется к корню, исключая ошибки потери пути.
  * У папок с точками в названии (например, `Отчет_2026.11`) часть после точки не воспринимается как расширение файла — имя каталога обрабатывается целиком.
* **Встроенный предпросмотр изображений, EXIF, текста и папок:**
  * **Одиночный клик** по имени объекта в таблице открывает его в карточке **«👁 Предпросмотр объекта»** на левой панели, а **двойной клик** открывает файл в системной программе Windows по умолчанию:
    * **Изображения (`jpg`, `jpeg`, `png`, `webp`, `bmp`, `ico`, `gif`):** масштабируемая миниатюра с указанием разрешения в пикселях, даты съёмки EXIF и модели камеры.
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

1. Скачайте портативный исполняемый файл по [**прямой ссылке (`restem_1.3.0.exe`)**](../../releases/latest/download/restem_1.3.0.exe) или со страницы [**Releases**](../../releases) (история версий в [**CHANGELOG.md**](CHANGELOG.md)).
2. Запустите `restem_1.3.0.exe` — установка не требуется.
3. Нажмите **«📁 Папка...»** (`Ctrl+O`), перетащите каталог мышью в окно программы или откройте папку через контекстное меню Проводника Windows.

---

### 📄 Лицензия

Распространяется под лицензией **MIT License**. Свободно для личного и коммерческого использования.

---
---

## 🇬🇧 ReStem — Fast Bulk File & Folder Renamer

**ReStem** is a lightweight, native desktop utility for batch renaming files and directories, built in **Rust** using the `eframe` (`egui`) immediate-mode GUI framework. It runs from a single portable `.exe` with zero external dependencies, launches instantaneously, and smoothly handles tens of thousands of items using virtual table rendering.

### ✨ Key Features (v1.3.0)

* **Smart Tags & Built-In EXIF Reader:**
  * Dynamically insert metadata variables into Prefix, Suffix, Index Insertion, and Replacement fields:
    * `{date}`, `{year}`, `{month}`, `{day}` — file modification date (`YYYY-MM-DD`).
    * `{exif_date}` — original photo capture date extracted directly from JPEG EXIF headers.
    * `{camera}` — camera or smartphone model from EXIF metadata (e.g., `Canon_EOS_R6`).
    * `{parent}` — parent directory name.
    * `{w}x{h}`, `{w}`, `{h}` — exact image dimensions in pixels (e.g., `1920x1080`) read instantaneously from image headers.
* **Real-Time Inline Character Diff:**
  * Toggle **🎨 Inline Diff** in the top toolbar to highlight removed characters in red strikethrough and newly inserted characters in green directly inside filenames.
* **Files, Folders & Recursive Subdirectories:**
  * Rename files, folders, or entire directory trees recursively using **Safe Bottom-Up Traversal** so parent folder renames never invalidate child paths.
  * Folder names containing dots (e.g., `Report_2026.11`) are treated as complete names rather than split into file extensions.
* **Built-In Image, EXIF, Text & Folder Preview:**
  * **Single-click** any item in the table to inspect it in the **👁 Item Preview** card, or **double-click** to open it in the default Windows application:
    * **Images (`jpg`, `jpeg`, `png`, `webp`, `bmp`, `ico`, `gif`):** proportional thumbnail preview with pixel dimensions, EXIF date, and camera model.
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

1. Download [**`restem_1.3.0.exe` (Direct Download)**](../../releases/latest/download/restem_1.3.0.exe) or visit the [**Releases**](../../releases) page (see [**CHANGELOG.md**](CHANGELOG.md) for version history).
2. Run `restem_1.3.0.exe` — no installation or runtime required.
3. Click **📁 Folder...** (`Ctrl+O`), drag and drop a folder into the window, or open any folder via the Windows Explorer context menu.

---

### 📄 License

Distributed under the **MIT License**. Free for personal and commercial use.
