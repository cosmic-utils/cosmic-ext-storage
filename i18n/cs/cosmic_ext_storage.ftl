app-title = Úložiště
settings = Nastavení
about = O aplikaci

git-description = Git commit {$hash} dne {$date}

# Menu items
new-disk-image = Nový obraz disku
attach-disk-image = Připojit obraz disku
create-disk-from-drive = Vytvořit disk z jednotky
create-image = Vytvořit obraz
restore-image-to-drive = Obnovit obraz na jednotku
restore-image = Obnovit obraz
create-disk-from-partition = Vytvořit obraz disku z oddílu
restore-image-to-partition = Obnovit obraz disku do oddílu
image-file-path = Cesta k souboru obrazu
image-destination-path = Cílová cesta souboru
image-source-path = Zdrojová cesta obrazu
image-size = Velikost obrazu
choose-path = Vybrat...
no-file-selected = Nebyl vybrán žádný soubor
attach = Připojit
restore-warning = Tímto přepíšete vybrané cílové zařízení. Tento krok nelze vrátit zpět.
eject = Vysunout
eject-failed = Vysunutí selhalo
power-off = Vypnout
power-off-failed = Vypnutí selhalo
format-disk = Naformátovat disk
format-disk-failed = Formátování disku selhalo
smart-data-self-tests = SMART data a vlastní testy
standby-now = Režim spánku nyní
standby-failed = Režim spánku selhal
wake-up-from-standby = Probudit ze spánku
wake-up-failed = Probuzení selhalo
unmount-failed = Odpojení selhalo

# Unmount busy dialog
unmount-busy-title-template = {$device} je obsazeno
unmount-busy-message-template = Následující procesy přistupují k {$mount}
unmount-busy-header-pid = PID
unmount-busy-header-command = Příkaz
unmount-busy-header-user = Uživatel
unmount-busy-no-processes = Zařízení je obsazeno, ale žádné procesy nebyly nalezeny. Zkuste to znovu nebo ručně zavřete otevřené soubory.
unmount-busy-kill-warning = Ukončení procesů může způsobit ztrátu nebo poškození dat.
unmount-busy-kill-and-retry = Ukončit procesy a zkusit znovu
retry = Zkusit znovu

# Dialog buttons
ok = Ok
cancel = Zrušit
continue = Pokračovat
working = Probíhá…

# Common
close = Zavřít
refresh = Obnovit
next = Další
behavior = Chování
credentials = Přihlašovací údaje
review = Kontrola
details = Podrobnosti

# Format disk dialog
erase-dont-overwrite-quick = Nepřepisovat (rychle)
erase-overwrite-slow = Přepsat (pomalu)
partitioning-dos-mbr = Kompatibilní s DOS/MBR
partitioning-gpt = Moderní (GPT)
partitioning-none = Žádné

# Create partition dialog
create-partition = Vytvořit oddíl
create-partition-failed = Vytvoření oddílu selhalo
format-partition = Naformátovat oddíl
format = Formátovat
format-partition-description = Tímto naformátujete vybraný svazek. Velikost: { $size }
volume-name = Název svazku
partition-name = Název oddílu
partition-size = Velikost oddílu
free-space = Volné místo
erase = Vymazat
password-protected = Chráněno heslem
password = Heslo
confirm = Potvrdit
password-required = Heslo je povinné.
password-mismatch = Hesla se neshodují.
apply = Použít
untitled = Bez názvu

# Main view
no-disk-selected = Nebyl vybrán žádný disk
no-volumes = Žádné dostupné svazky
partition-number = Oddíl { $number }
partition-number-with-name = Oddíl { $number }: { $name }
volumes = Svazky
unknown = Neznámé
unresolved = Nevyřešeno

# Info labels
size = Velikost
usage = Využití
mounted-at = Připojeno v
contents = Obsah
device = Zařízení
partition = Oddíl
path = Cesta
uuid = UUID
model = Model
serial = Sériové číslo
partitioning = Rozdělení oddílů
backing-file = Základní soubor

# Confirmation dialog
delete = Smazat { $name }
delete-partition = Smazat
delete-confirmation = Opravdu chcete smazat { $name }?
delete-failed = Smazání selhalo

# Volume segments
free-space-segment = Volné místo
reserved-space-segment = Rezervováno
filesystem = Souborový systém
free-space-caption = Volné místo
reserved-space-caption = Rezervované místo

# Encrypted / LUKS
unlock-button = Odemknout
lock = Zamknout
unlock = Odemknout { $name }
passphrase = Přístupová fráze
current-passphrase = Aktuální přístupová fráze
new-passphrase = Nová přístupová fráze
change-passphrase = Změnit přístupovou frázi
passphrase-mismatch = Přístupové fráze se neshodují.
locked = Zamčeno
unlocked = Odemčeno
unlock-failed = Odemknutí selhalo
lock-failed = Zamčení selhalo
unlock-missing-partition = Nepodařilo se nalézt { $name } v aktuálním seznamu zařízení.

# Volume commands
mount = Připojit
unmount = Odpojit
operation-cancelled = Operace zrušena
edit-mount-options = Upravit možnosti připojení…
edit-mount-options-failed = Úprava možností připojení selhala
edit-encryption-options = Upravit možnosti šifrování…
edit-partition = Upravit oddíl
edit = Upravit
edit-partition-no-types = Pro tuto tabulku oddílů nejsou k dispozici žádné typy oddílů.
flag-legacy-bios-bootable = Bootovatelný v Legacy BIOS
flag-system-partition = Systémový oddíl
flag-hide-from-firmware = Skrýt před firmware
resize-partition = Změnit velikost oddílu
resize = Změnit velikost
resize-partition-range = Povolený rozsah: { $min } až { $max }
new-size = Nová velikost
edit-filesystem = Upravit souborový systém
label = Štítek
filesystem-label = Štítek souborového systému
check-filesystem = Zkontrolovat souborový systém
check-filesystem-warning = Kontrola souborového systému může trvat dlouho. Pokračovat?
repair-filesystem = Opravit souborový systém
repair = Opravit
repair-filesystem-warning = Oprava souborového systému může trvat dlouho a hrozí ztráta dat. Pokračovat?
take-ownership = Převzít vlastnictví
take-ownership-warning = Tímto změníte vlastníka souborů na svého uživatele. Může to trvat dlouho a nelze to snadno vrátit zpět.
take-ownership-recursive = Použít rekurzivně

# Mount/encryption options
user-session-defaults = Výchozí hodnoty uživatelské relace
mount-at-startup = Připojit při startu systému
unlock-at-startup = Odemknout při startu systému
require-auth-to-mount = Vyžadovat autorizaci pro připojení nebo odpojení
require-auth-to-unlock = Vyžadovat autorizaci pro odemknutí
show-in-ui = Zobrazit v uživatelském rozhraní
identify-as = Identifikovat jako
other-options = Další možnosti
mount-point = Bod připojení
filesystem-type = Typ souborového systému
display-name = Zobrazovaný název
icon-name = Název ikony
symbolic-icon-name = Název symbolické ikony
show-passphrase = Zobrazit přístupovou frázi
name = Název

# SMART
smart-no-data = Žádná SMART data nejsou k dispozici.
smart-type = Typ
smart-updated = Aktualizováno
smart-temperature = Teplota
smart-power-on-hours = Hodiny provozu
smart-selftest = Vlastní test
smart-selftest-short = Krátký vlastní test
smart-selftest-extended = Rozšířený vlastní test
smart-selftest-abort = Přerušit vlastní test

# Volume types
lvm-logical-volume = LVM LV
lvm-physical-volume = LVM PV
luks-container = LUKS
partition-type = Oddíl
block-device = Zařízení

# Status
not-mounted = Nepřipojeno
can-create-partition = Lze vytvořit oddíl

# Filesystem tools detection
fs-tools-missing-title = Chybějící nástroje souborového systému
fs-tools-missing-desc = Následující nástroje nejsou nainstalovány. Nainstalujte je pro plnou podporu souborových systémů:
fs-tools-all-installed-title = Nástroje souborového systému
fs-tools-all-installed = Všechny nástroje souborového systému jsou nainstalovány.
fs-tools-required-for = potřebné pro podporu {$fs_name}

# UDisks2 BTRFS module
settings-enable-ustorage-btrfs = Pokusit se povolit UDisks2 BTRFS
settings-ustorage-btrfs-enabled = Modul UDisks2 BTRFS povolen
settings-ustorage-btrfs-enabled-body = Modul UDisks2 BTRFS byl úspěšně povolen. Nyní můžete používat funkce správy BTRFS.
settings-ustorage-btrfs-failed = Povolení modulu UDisks2 BTRFS selhalo

offset = Posun

# Partition dialog labels
overwrite-data-slow = Přepsat data (pomalu)
password-protected-luks = Chráněno heslem (LUKS)

# Filesystem type names
fs-name-ext4 = ext4
fs-name-ext3 = ext3
fs-name-xfs = XFS
fs-name-btrfs = Btrfs
fs-name-f2fs = F2FS
fs-name-udf = UDF
fs-name-ntfs = NTFS
fs-name-vfat = FAT32
fs-name-exfat = exFAT
fs-name-swap = Swap

# Filesystem type descriptions
fs-desc-ext4 = Moderní Linuxový souborový systém (výchozí)
fs-desc-ext3 = Legacy Linuxový souborový systém
fs-desc-xfs = Výkonné journalingové FS
fs-desc-btrfs = Copy-on-write s podporou snapshotů
fs-desc-f2fs = Souborový systém optimalizovaný pro flash
fs-desc-udf = Univerzální formát disku
fs-desc-ntfs = Souborový systém Windows
fs-desc-vfat = Univerzální kompatibilita
fs-desc-exfat = Velké soubory, multiplatformní
fs-desc-swap = Virtuální paměť

# Filesystem tools warning
fs-tools-warning = Některé typy souborových systémů chybí kvůli chybějícím nástrojům. Více informací v Nastavení.

# Detail Tabs
volume-info = Informace o svazku

# BTRFS Management
btrfs-management = Správa BTRFS
btrfs = BTRFS
volume = Svazek
btrfs-placeholder = Funkce správy BTRFS budou brzy k dispozici
btrfs-create-subvolume = Vytvořit podsvazek
btrfs-subvolume-name = Název podsvazku
btrfs-subvolume-name-required = Název podsvazku je povinný
btrfs-subvolume-invalid-chars = Název podsvazku nesmí obsahovat lomítka
btrfs-create-subvolume-failed = Vytvoření podsvazku selhalo
btrfs-delete-subvolume = Smazat podsvazek
btrfs-delete-confirm = Smazat podsvazek '{ $name }'? Tuto akci nelze vrátit.
btrfs-delete-subvolume-failed = Smazání podsvazku selhalo
btrfs-create-snapshot = Vytvořit snapshot
btrfs-source-subvolume = Zdrojový podsvazek
btrfs-snapshot-name = Název snapshotu
btrfs-read-only = Snapshot jen pro čtení
btrfs-create-snapshot-failed = Vytvoření snapshotu selhalo
btrfs-used-space = Využité místo
btrfs-subvolume-id = ID
btrfs-subvolume-path = Cesta
btrfs-subvolume-actions = Akce
btrfs-set-default-failed = Nastavení výchozího podsvazku selhalo
btrfs-readonly-failed = Přepnutí do režimu jen pro čtení selhalo
btrfs-not-mounted = Souborový systém BTRFS není připojen
btrfs-not-mounted-refresh = Souborový systém BTRFS není připojen (zkuste obnovit)
btrfs-loading-subvolumes = Načítání podsvazků...
btrfs-no-subvolumes = Nenašly se žádné podsvazky
btrfs-no-subvolumes-desc = Tento BTRFS svazek může být nově vytvořený nebo ještě nemá žádné podsvazky.
btrfs-loading-usage = Načítání informací o využití...
btrfs-usage-error = Chyba využití: { $error }

# Usage view
usage-scanning = Zjišťování využití disku...
usage-scan-failed = Analýza využití selhala
usage-scan-not-started = Analýza využití nebyla spuštěna
usage-files-per-category = Soubory podle kategorie
usage-filename = Název souboru
usage-selected-count = Vybráno: { $count }
usage-clear-selection = Zrušit výběr
usage-configure = Nastavit
usage-show-all-root-mode = Zobrazit všechny soubory (režim root)
usage-scan-setup = Nastavení analýzy využití
usage-choose-mount-points = Vyberte přípojné body
usage-choose-mount-points-desc = Vyberte jeden nebo více přípojných bodů, které budou zahrnuty do analýzy.
usage-scan-parallelism-label = Paralelismus analýzy
usage-parallelism-low = Nízký
usage-parallelism-balanced = Vyvážený
usage-parallelism-high = Vysoký
usage-selected = Vybráno
usage-not-selected = Nevybráno
usage-loading-mount-points = Načítání přípojných bodů...
usage-no-mount-points = Nejsou k dispozici žádné přípojné body.
usage-parallelism = Paralelismus
usage-start-scan = Spustit analýzu
usage-select-at-least-one-mount-point = Vyberte alespoň jeden přípojný bod
usage-delete-summary = Smazáno souborů: { $deleted }; neúspěšné: { $failed }

# Usage categories
usage-category-documents = Dokumenty
usage-category-images = Obrázky
usage-category-audio = Zvuk
usage-category-video = Video
usage-category-archives = Archivy
usage-category-code = Kód
usage-category-binaries = Binární soubory
usage-category-packages = Balíčky
usage-category-system = Systém
usage-category-other = Ostatní
