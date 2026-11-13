<div align="center">



# ReStem

**Быстрое пакетное переименование файлов и каталогов на Rust**  
**Fast Bulk File & Folder Renamer written in Rust**

[![Version: v1.2.0](https://img.shields.io/badge/Version-v1.2.0-2ea44f?logo=github)](CHANGELOG.md)
[![Language: Rust](https://img.shields.io/badge/Rust-2021_Edition-orange?logo=rust)](https://www.rust-lang.org/)
[![GUI: egui](https://img.shields.io/badge/GUI-eframe_%2F_egui-00b4d8)](https://github.com/emilk/egui)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows_x64-0078d4?logo=windows)](../../releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-success)](LICENSE)

[**🇷🇺 Русская документация**](#-restem--пакетное-переименование-файлов-и-папок) &nbsp;|&nbsp; [**🇬🇧 English Documentation**](#-restem--fast-bulk-file--folder-renamer) &nbsp;|&nbsp; [**📋 История изменений (Changelog)**](CHANGELOG.md)
<br>

[![Скачать ReStem v1.2.0](https://img.shields.io/badge/⬇_Скачать_ReStem_v1.2.0-Прямая_ссылка_(.exe)-0078d4?style=for-the-badge&logo=windows&logoColor=white)](../../releases/latest/download/restem_1.2.0.exe)

<br>

[**🇷🇺 Русская документация**](#-restem--пакетное-переименование-файлов-и-папок) &nbsp;|&nbsp; [**🇬🇧 English Documentation**](#-restem--fast-bulk-file--folder-renamer) &nbsp;|&nbsp; [**📋 История изменений (Changelog)**](CHANGELOG.md)
</div>

---

## 🇷🇺 ReStem — Пакетное переименование файлов и папок

**ReStem** — это легковесная нативная десктопная утилита для пакетного переименования файлов и каталогов, написанная на языке **Rust** с использованием графического движка `eframe` (`egui`). Программа работает из единого портативного `.exe` файла, запускается за миллисекунды, не требует установки сторонних библиотек и плавно отображает десятки тысяч объектов благодаря виртуальной таблице.

### ✨ Ключевые возможности

* **Работа с файлами, папками и вложенными подкаталогами:**
  * Гибкая фильтрация объектов: переименовывайте только файлы, только папки или всё дерево каталогов рекурсивно.
  * **Безопасный иерархический обход (Bottom-Up):** при переименовании вложенных структур программа автоматически начинает с самого глубокого уровня вложенности и движется к корню, исключая ошибки потери пути.
  * У папок с точками в названии (например, `Отчет_2026.11`) часть после точки не воспринимается как расширение файла — имя каталога обрабатывается целиком.
* **Встроенный предпросмотр изображений, текста и папок:**
  * Клик по любому имени объекта в таблице открывает его в карточке **«👁 Предпросмотр объекта»** на левой панели:
    * **Изображения (`jpg`, `jpeg`, `png`, `webp`, `bmp`, `ico`, `gif`):** отображаются в виде масштабируемой миниатюры с указанием точного разрешения в пикселях (например, `1920 × 1080 px`).
    * **Текстовые файлы и исходный код (`txt`, `md`, `json`, `csv`, `log`, `ini`, `rs`, `py` и др.):** мгновенный предпросмотр первых 4 КБ содержимого моноширинным шрифтом `Consolas` с автоматическим распознаванием бинарных файлов.
    * **Папки:** отображение количества вложенных объектов и кнопка быстрого перехода внутрь каталога.
* **Безопасное удаление в Корзину Windows:**
  * **Пакетное удаление:** кнопка **«🗑 В корзину (N)»** на верхней панели инструментов перемещает все отмеченные галочками файлы и папки в стандартную Корзину Windows после подтверждения в диалоговом окне.
  * **Одиночное удаление:** кнопка быстрого удаления выбранного файла или папки прямо внутри карточки предпросмотра.
* **Предпросмотр новых имён в реальном времени и точечный выбор:**
  * Любое изменение правил мгновенно отображается в колонке **«Новое имя»** с контрастной цветовой подсветкой.
  * Индивидуальные галочки (чекбоксы) напротив каждой строки позволяют исключить отдельные файлы или папки из пакетной обработки, а кнопка **«☑ Инвертировать»** и главный чекбокс в шапке таблицы ускоряют массовый выбор.
  * Сквозной счётчик нумерации автоматически пропускает снятые с выбора строки, не создавая пропусков в последовательности номеров.
* **Гибкая сортировка списка:**
  * **Умная сортировка по имени (Natural Sort: `1, 2, 10`):** распознаёт числа внутри строк (`Кадр_2.jpg` идёт перед `Кадр_10.jpg`).
  * Сортировка по **алфавиту (`А–Я`)**, **дате изменения**, **размеру файла**, **расширению/типу** и **расположению (пути)**.
  * Быстрая сортировка в один клик по заголовкам столбцов таблицы (`Тип`, `Текущее имя`, `Размер`, `Изменён`), переключение направления (`▲ По возр.` / `▼ По убыв.`) и опция фиксации **«Папки сверху»**.
* **Многоуровневая фильтрация и защита служебных папок:**
  * Фильтр отображения по статусу: *Все объекты*, *Только изменяемые*, *Только выбранные галочкой ☑*, *Без изменений*.
  * Поиск по имени в верхней панели с поддержкой **регулярных выражений (`.*`)** и учёта регистра (`Aa`).
  * Автоматическое скрытие системных и скрытых файлов Windows, а также служебных каталогов (`.git`, `node_modules`, `$RECYCLE.BIN`).
* **Нативная типографика Windows и карточный интерфейс (v1.2.0):**
  * Использование системных шрифтов Windows (`Segoe UI`, `Consolas`, `Segoe UI Emoji`) с повышенной контрастностью текста и скруглёнными карточками модулей.
  * Мгновенное переключение языка интерфейса **RU / EN** и переключение между **☀ Светлой** и **🌙 Тёмной** темой в один клик.
  * Информационное окно **«ℹ О программе»** с данными о версии сборки и копированием версии в буфер обмена.
* **Поддержка Drag-and-Drop и откат (Undo):**
  * Перетащите любую папку или файл из Проводника Windows прямо в окно ReStem для мгновенной загрузки списка.
  * Автоматическая проверка конфликтов и дубликатов имён до начала переименования и кнопка **«↺ Откат (Undo)»** для возврата прежних имён.

---

### 🛠 Правила переименования и фильтры (Правая панель)

1. **Поиск и замена (Find & Replace):**
   * Замена текста с поддержкой **регулярных выражений (Regex)** и группами захвата (`$1`, `$2`).
   * Опция учёта регистра символов.
2. **Регистр и транслитерация (Case & Transliteration):**
   * Режимы регистра: `Не менять регистр` / `все строчные` / `ВСЕ ПРОПИСНЫЕ` / `Первая буква заглавная` / `Каждое Слово С Заглавной` с полной поддержкой кириллицы и UTF-8.
   * **Транслитерация кириллицы (`RU → LAT`):** автоматический перевод русских букв в латиницу (`ж → zh`, `ч → ch`, `ш → sh`, `щ → shch`, `ю → yu`, `я → ya`, `ъ/ь` опускаются) с сохранением регистра.
3. **Префикс, суффикс и обрезка (Add & Trim):**
   * Добавление произвольного текста в начало имени или перед расширением файла.
   * Посимвольная обрезка заданного количества знаков слева и справа.
4. **Автонумерация (Numbering):**
   * Добавление номера в начало (префикс) или в конец имени (суффикс) **строго в порядке текущей сортировки таблицы**.
   * Настройка стартового числа, шага, разделителя и количества ведущих нулей (`01`, `001`, `0001`), а также **«Сброс счётчика в подпапках»**.
5. **Расширения файлов (File Extensions):**
   * Перевод расширений в `строчные (.jpg)`, `ПРОПИСНЫЕ (.JPG)` или полное удаление расширения.
   * Пакетная замена текущего расширения на новое (например, `jpeg` → `jpg`). На имена папок правила расширений не влияют.
6. **Фильтры и исключения (Advanced Filters):**
   * **Белый и чёрный списки расширений:** включение (`jpg, png`) или исключение (`tmp, bak`) форматов.
   * **Исключение по словам:** скрытие объектов, содержащих заданные слова через запятую.
   * **Фильтр по размеру файла:** выбор файлов *Больше чем N КБ*, *Меньше чем N КБ* или в *Диапазоне от N до M КБ*.

---

### 🚀 Скачивание и запуск

Готовый исполняемый файл не требует установки дополнительных библиотек:
1. Перейдите в раздел [**Releases**](../../releases) и скачайте `restem.exe` (список нововведений доступен в [**CHANGELOG.md**](CHANGELOG.md)).
2. Запустите `restem.exe`.
3. Нажмите **«📁 Открыть папку...»** или перетащите нужный каталог мышью в окно программы.

---

### 📄 Лицензия

Распространяется под лицензией **MIT License**. Свободно для личного и коммерческого использования.

---
---

## 🇬🇧 ReStem — Fast Bulk File & Folder Renamer

**ReStem** is a lightweight, native desktop utility for batch renaming files and directories, built in **Rust** using the `eframe` (`egui`) immediate-mode GUI framework. It runs from a single portable `.exe` with zero external dependencies, launches instantaneously, and smoothly handles tens of thousands of items using virtual table rendering.

### ✨ Key Features

* **Files, Folders & Recursive Subdirectories:**
  * Choose whether to rename files, folders, or entire directory trees recursively.
  * **Safe Bottom-Up Traversal:** when renaming nested directories and files simultaneously, ReStem automatically sorts operations from the deepest nesting level up to the root so parent folder renames never break child paths.
  * Folder names containing dots (e.g., `Report_2026.11`) are treated as complete names rather than split into file extensions.
* **Built-In Image, Text & Folder Preview:**
  * Clicking any item name in the table opens it in the **👁 Item Preview** card on the left panel:
    * **Images (`jpg`, `jpeg`, `png`, `webp`, `bmp`, `ico`, `gif`):** renders a proportional thumbnail preview alongside exact pixel dimensions (e.g., `1920 × 1080 px`).
    * **Text & Code Files (`txt`, `md`, `json`, `csv`, `log`, `ini`, `rs`, `py`, etc.):** instant preview of the first 4 KB of text in `Consolas` monospace font with automatic binary file detection.
    * **Folders:** displays the number of contained items and a quick button to enter the directory.
* **Safe Deletion to Windows Recycle Bin:**
  * **Batch Trash:** the **🗑 Trash (N)** button in the top toolbar moves all checked files and folders to the Windows Recycle Bin after a confirmation prompt.
  * **Single-Item Trash:** delete the currently inspected file or folder directly from the preview card.
* **Real-Time Preview & Granular Selection:**
  * Every rule adjustment updates the **New Name** column in real time with pill-style color highlighting.
  * Individual checkboxes next to every row let you exclude specific items, while the master header checkbox and **☑ Invert** button speed up batch selection.
  * Sequential numbering automatically skips unchecked rows so your numbering sequence stays contiguous without gaps.
* **Multi-Mode Sorting:**
  * **Natural Name Sort (`1, 2, 10`):** smart numerical sorting (`Frame_2.jpg` comes before `Frame_10.jpg`).
  * Sort by **Alphabetical (`A–Z`)**, **Modified Date**, **File Size**, **Extension / Type**, or **Folder Path**.
  * Clickable table column headers (`Type`, `Original Name`, `Size`, `Modified`), ascending/descending toggle (`▲ Asc` / `▼ Desc`), and **Folders first** pinning.
* **Advanced Filtering & System File Protection:**
  * Filter rows by status: *All items*, *Only changing*, *Only checked ☑*, or *Unchanged only*.
  * Top toolbar search bar with **Regular Expression (`.*`)** and **Case-Sensitive (`Aa`)** toggles.
  * Automatic filtering of hidden/system Windows files and service directories (`.git
