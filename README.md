# DeepSearch

```
▐▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▌
▐*██████╗*███████╗███████╗██████╗*****███████╗███████╗*█████╗*██████╗**██████╗██╗**██╗*▌
▐*██╔══██╗██╔════╝██╔════╝██╔══██╗****██╔════╝██╔════╝██╔══██╗██╔══██╗██╔════╝██║**██║*▌
▐*██║**██║█████╗**█████╗**██████╔╝****███████╗█████╗**███████║██████╔╝██║*****███████║*▌
▐*██║**██║██╔══╝**██╔══╝**██╔═══╝*****╚════██║██╔══╝**██╔══██║██╔══██╗██║*****██╔══██║*▌
▐*██████╔╝███████╗███████╗██║*********███████║███████╗██║**██║██║**██║╚██████╗██║**██║*▌
▐*╚═════╝*╚══════╝╚══════╝╚═╝*********╚══════╝╚══════╝╚═╝**╚═╝╚═╝**╚═╝*╚═════╝╚═╝**╚═╝*▌
▐▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▌
```

**DeepSearch** is a high-performance file indexing and search utility developed in Rust.  
It builds a fast, searchable index of your local folders and network shared directories (SMB), allowing for near-instantaneous file name lookups.

---

## Features

- **Persistent Indexing**: Uses a local `redb` database to store file indexes, allowing for fast subsequent searches without needing to re-scan entire directories.
- **Incremental Updates**: A "Rescan" feature intelligently compares directories against the existing index to find and apply only the changes (new, modified, or deleted files).
- **Optimized for Performance**: Leverages Rust’s concurrency model (`rayon`) for both indexing and searching. The thread pool is tuned for I/O-bound workloads like network shares to maximize throughput.
- **Intelligent Search**: File name search is insensitive to case and diacritics (e.g., `thanh` will match `Thành`).
- **Recursive Scanning**: Indexes all files in all subdirectories of a given path.
- **Modern CLI**: An interactive command-line interface with a "neo-tech" style, featuring colors and progress bars for a clear and responsive user experience.

---

## Usage Guide

### Core Concept

DeepSearch works in two main phases: **1. Indexing** and **2. Searching**. You must first index a directory to make its contents searchable.

### 1. Initial Scan (Indexing a new directory)

This is the first step for any new directory you want to make searchable.

1.  Launch the application (`DeepSearch.exe`).
2.  Select `1. Initial Scan` from the main menu.
3.  When prompted, enter the full path to the directory you want to index (e.g., `C:\Users\YourName\Documents` or `\\server\share`).
4.  The application will scan the entire directory and build a searchable index. A progress bar will show the status of the scan.

### 2. Searching the Index

Once one or more directories have been indexed, you can search them.

1.  From the main menu, select `3. Search`.
2.  A menu will appear showing all previously indexed locations. Select the scope you want to search in (e.g., a specific location or all indexed locations).
3.  When prompted, enter your search keyword.
4.  Results matching the file name will be displayed instantly.

### 3. Updating an Index (Rescan)

If the contents of an indexed directory have changed (files added, deleted, or modified), run a Rescan to update the index.

1.  From the main menu, select `2. Rescan`.
2.  A menu will appear showing all previously indexed locations. Choose the path you want to update.
3.  The application will re-scan the directory, compare it with the old index, and efficiently save only the changes.

---

## Preview

Below is a representation of the application flow.

**Main Menu:**
```
--- DeepSearch Main Menu ---
1. Initial Scan
2. Rescan
3. Search
q. Quit
Select an option: 
```

**Scanning with Progress Bar:**
```
🔍 Starting initial scan for '\some\network\share'...
Phase 1/2: Discovering directories...
Phase 2/2: Scanning files...
[00:00:07] [████████████████████>------------------] 1325/2400 (ETA: 00:00:06) Phase 2/2: Scanning files...
```

**Search Workflow:**
```
--- Select Search Scope ---
1. C:\Users\YourName\Documents
2. \\some\network\share
a. All
⌨️ Select a location to search in (1, 2, ..., a): 2

⌨️ Enter search keyword: report 2025

🔍 Searching for 'report 2025' in the selected scope...

--- Search Results (3 found) ---
\\some\network\share\archive\Report 2025 Final.docx
\\some\network\share\drafts\report_2025_v2.pdf
\\some\network\share\project-x\report-2025.xlsx
```

---

## Development Roadmap

Planned enhancements in upcoming versions:

- Regular expression (regex) search support  
- Exporting search results to CSV  
- Optional dark mode theme  
- Type-ahead file name suggestions  

---

## Contribution and Contact

For feedback, suggestions, or bug reports, please:

- Submit an issue on the project’s GitHub page (link to be provided), or  
- Contact the author directly.

---

_DeepSearch is built with performance, reliability, and usability in mind — for developers, analysts, and IT professionals.