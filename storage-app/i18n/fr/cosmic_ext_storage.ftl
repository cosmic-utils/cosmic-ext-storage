app-title = Stockage
settings = Paramètres
about = À propos

git-description = Git commit {$hash} le {$date}

# Éléments du menu
new-disk-image = Nouvelle image disque
attach-disk-image = Joindre une image disque
create-disk-from-drive = Créer un disque à partir d'un lecteur
create-image = Créer une image
restore-image-to-drive = Restaurer une image sur un lecteur
restore-image = Restaurer une image
create-disk-from-partition = Créer une image disque à partir d'une partition
restore-image-to-partition = Restaurer une image disque sur une partition
image-file-path = Chemin d'accès au fichier image
image-destination-path = Chemin d'accès au fichier de destination
image-source-path = Chemin d'accès à l'image source
image-size = Taille de l'image
choose-path = Choisir...
no-file-selected = Aucun fichier sélectionné
attach = Joindre
restore-warning = Cette opération écrasera le périphérique cible sélectionné. Elle est irréversible.
eject = Éjecter
eject-failed = Échec de l'éjection
power-off = Mise hors tension
power-off-failed = Échec de la mise hors tension
format-disk = Formatage du disque
format-disk-failed = Échec du formatage du disque
smart-data-self-tests = Données SMART et autotests
standby-now = Mise en veille
standby-failed = Échec de la mise en veille
wake-up-from-standby = Sortie de veille
wake-up-failed = Échec de la sortie de veille
unmount-failed = Échec du démontage

# Boîte de dialogue Démontage
unmount-busy-title-template = Le périphérique {$device} est occupé
unmount-busy-message-template = Les processus suivants accèdent au périphérique {$mount}
unmount-busy-header-pid = PID
unmount-busy-header-command = Commande
unmount-busy-header-user = Utilisateur
unmount-busy-no-processes = Le périphérique est occupé, mais aucun processus n'a été trouvé. Veuillez réessayer ou fermer manuellement les fichiers.
unmount-busy-kill-warning = L'arrêt brutal des processus peut entraîner une perte ou une corruption de données.
unmount-busy-kill-and-retry = Arrêter les processus et réessayer
retry = Réessayer

# Boîte de dialogue Boutons
ok = OK
cancel = Annuler
continue = Continuer
working = Traitement en cours…

# Commun
close = Fermer
refresh = Actualiser
next = Suivant
behavior = Comportement
credentials = Identifiants
review = Révision
details = Détails

# Boîte de dialogue Formater le disque
erase-dont-overwrite-quick = Ne pas écraser (Rapide)
erase-overwrite-slow = Écraser (Lent)
partitioning-dos-mbr = Compatible avec les anciens systèmes (DOS/MBR)
partitioning-gpt = Moderne (GPT)
partitioning-none = Aucun

# Boîte de dialogue Créer une partition
create-partition = Créer une partition
create-partition-failed = Échec de la création de la partition
format-partition = Formater la partition
format = Formater
format-partition-description = Cette action formatera le volume sélectionné. Taille : { $size }
volume-name = Nom du volume
partition-name = Nom de la partition
partition-size = Taille de la partition
free-space = Espace libre
erase = Effacer
password-protected = Protégé par mot de passe
password = Mot de passe
confirm = Confirmer
password-required = Un mot de passe est requis.
password-mismatch = Les mots de passe ne correspondent pas.
apply = Appliquer
untitled = Sans titre

# Vue principale
no-disk-selected = Aucun disque sélectionné
no-volumes = Aucun volume disponible
partition-number = Partition { $number }
partition-number-with-name = Partition { $number } : { $name }
volumes = Volumes
unknown = Inconnu
unresolved = Non résolu

# Étiquettes d'information
size = Taille
usage = Utilisation
mounted-at = Monté sur
contents = Contenu
device = Périphérique
partition = Partition
path = Chemin
uuid = UUID
model = Modèle
serial = Numéro de série
partitioning = Partitionnement
backing-file = Fichier de sauvegarde

# Boîte de dialogue Confirmation
delete = Supprimer { $name }
delete-partition = Supprimer
delete-confirmation = Êtes-vous sûr de vouloir supprimer { $name } ?
delete-failed = Échec de la suppression

# Segments de volume
free-space-segment = Espace libre
reserved-space-segment = Réservé
filesystem = Système de fichiers
free-space-caption = Espace libre
reserved-space-caption = Espace réservé

# Chiffré / LUKS
unlock-button = Déverrouiller
lock = Verrouiller
unlock = Déverrouiller { $name }
passphrase = Phrase secrète
current-passphrase = Phrase secrète actuelle
new-passphrase = Nouvelle phrase secrète
change-passphrase = Modifier la phrase secrète
passphrase-mismatch = Les phrases secrètes ne correspondent pas.
locked = Verrouillé
unlocked = Déverrouillé
unlock-failed = Échec du déverrouillage
lock-failed = Échec du verrouillage
unlock-missing-partition = Impossible de trouver { $name } dans la liste des périphériques.

# Commandes de volume
mount = Monter
unmount = Démonter
operation-cancelled = Opération annulée
edit-mount-options = Modifier les options de montage…
edit-mount-options-failed = Échec de la modification des options de montage
edit-encryption-options = Modifier les options de chiffrement…
edit-partition = Modifier la partition
edit = Modifier
edit-partition-no-types = Aucun type de partition disponible pour cette table de partitions.
flag-legacy-bios-bootable = BIOS hérité amorçable
flag-system-partition = Partition système
flag-hide-from-firmware = Masquer pour le firmware
resize-partition = Redimensionner la partition
resize = Redimensionner
resize-partition-range = Plage autorisée : { $min } à { $max }
new-size = Nouvelle taille
edit-filesystem = Modifier le système de fichiers
label = Étiquette
filesystem-label = Étiquette du système de fichiers
check-filesystem = Vérifier le système de fichiers
check-filesystem-warning = La vérification d'un système de fichiers peut prendre du temps. Continuer ?
repair-filesystem = Réparer le système de fichiers
repair = Réparer
repair-filesystem-warning = La réparation d'un système de fichiers peut prendre du temps et entraîner une perte de données. Continuer ?
take-ownership = S'approprier les fichiers
take-ownership-warning = Cette action transférera la propriété des fichiers à votre utilisateur. Cette opération peut prendre du temps et ne peut pas être facilement annulée.
take-ownership-recursive = Appliquer récursivement

# Options de montage/chiffrement
user-session-defaults = Paramètres par défaut de la session utilisateur
mount-at-startup = Monter au démarrage du système
unlock-at-startup = Déverrouiller au démarrage du système
require-auth-to-mount = Autorisation requise pour monter ou démonter
require-auth-to-unlock = Autorisation requise pour déverrouiller
show-in-ui = Afficher dans l'interface utilisateur
identify-as = Identifier comme
other-options = Autres options
mount-point = Point de montage
filesystem-type = Type de système de fichiers
display-name = Nom d'affichage
icon-name = Nom de l'icône
symbolic-icon-name = Nom symbolique de l'icône
show-passphrase = Afficher la phrase secrète
name = Nom

# SMART
smart-no-data = Aucune donnée SMART disponible.
smart-type = Type
smart-updated = Mise à jour
smart-temperature = Température
smart-power-on-hours = Heures de fonctionnement
smart-selftest = Autotest
smart-selftest-short = Autotest court
smart-selftest-extended = Autotest étendu
smart-selftest-abort = Annuler l'autotest

# Types de volumes
lvm-logical-volume = LVM LV
lvm-physical-volume = LVM PV
luks-container = LUKS
partition-type = Partition
block-device = Périphérique

# État
not-mounted = Non monté
can-create-partition = Création de partition possible

# Détection des outils du système de fichiers
fs-tools-missing-title = Outils du système de fichiers manquants
fs-tools-missing-desc = Les outils suivants ne sont pas installés. Installez-les pour activer la prise en charge complète du système de fichiers :
fs-tools-all-installed-title = Outils du système de fichiers
fs-tools-all-installed = Tous les outils du système de fichiers sont installés.
fs-tools-required-for = Requis pour la prise en charge de {$fs_name}

# Module BTRFS UDisks2
settings-enable-ustorage-btrfs = Tentative d'activation du module BTRFS UDisks2
settings-ustorage-btrfs-enabled = Module BTRFS UDisks2 activé
settings-ustorage-btrfs-enabled-body = Le module BTRFS UDisks2 a été activé avec succès. Vous pouvez désormais utiliser les fonctionnalités de gestion BTRFS.
settings-ustorage-btrfs-failed = Échec de l'activation du module BTRFS UDisks2

offset = Décalage

# Boîte de dialogue Étiquettes de partitionnement
overwrite-data-slow = Écraser les données (Lent)
password-protected-luks = Protégé par mot de passe (LUKS)

# Noms des types de systèmes de fichiers
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

# Descriptions des types de systèmes de fichiers
fs-desc-ext4 = Système de fichiers Linux moderne (par défaut)
fs-desc-ext3 = Système de fichiers Linux hérité
fs-desc-xfs = Journalisation haute performance
fs-desc-btrfs = Copie à l'écriture avec instantanés
fs-desc-f2fs = Système de fichiers optimisé pour la mémoire flash
fs-desc-udf = Format de disque universel
fs-desc-ntfs = Système de fichiers Windows
fs-desc-vfat = Compatibilité universelle
fs-desc-exfat = Fichiers volumineux, multiplateforme
fs-desc-swap = Mémoire virtuelle

# Avertissement concernant les outils de système de fichiers
fs-tools-warning = Certains types de systèmes de fichiers sont manquants en raison d'outils incomplets. Consultez les paramètres pour plus d'informations.

# Onglets de détail
volume-info = Informations sur le volume

# Gestion BTRFS
btrfs-management = Gestion BTRFS
btrfs = BTRFS
volume = Volume
btrfs-placeholder = Fonctionnalités de gestion BTRFS bientôt disponibles
btrfs-create-subvolume = Créer un sous-volume
btrfs-subvolume-name = Nom du sous-volume
btrfs-subvolume-name-required = Le nom du sous-volume est obligatoire
btrfs-subvolume-invalid-chars = Le nom du sous-volume ne peut pas contenir de barres obliques
btrfs-create-subvolume-failed = Échec de la création du sous-volume
btrfs-delete-subvolume = Supprimer le sous-volume
btrfs-delete-confirm = Supprimer le sous-volume « { $name } » ? Cette action est irréversible.
btrfs-delete-subvolume-failed = Échec de la suppression du sous-volume
btrfs-create-snapshot = Créer un instantané
btrfs-source-subvolume = Sous-volume source
btrfs-snapshot-name = Nom de l'instantané
btrfs-read-only = Instantané en lecture seule
btrfs-create-snapshot-failed = Échec de la création de l'instantané
btrfs-used-space = Espace utilisé
btrfs-subvolume-id = ID
btrfs-subvolume-path = Chemin
btrfs-subvolume-actions = Actions
btrfs-set-default-failed = Échec de la définition du sous-volume par défaut
btrfs-readonly-failed = Échec de l'activation/désactivation de l'indicateur de lecture seule
btrfs-not-mounted = Système de fichiers BTRFS non monté
btrfs-not-mounted-refresh = Système de fichiers BTRFS non monté (essayez d'actualiser)
btrfs-loading-subvolumes = Chargement des sous-volumes…
btrfs-no-subvolumes = Aucun sous-volume trouvé
btrfs-no-subvolumes-desc = Ce volume BTRFS est peut-être récent ou ne contient pas encore de sous-volumes.
btrfs-loading-usage = Chargement des informations d'utilisation…
btrfs-usage-error = Erreur d'utilisation : { $error }

# Vue d'utilisation
usage-scanning = Analyse de l'utilisation du disque...
usage-scan-failed = Échec de l'analyse d'utilisation
usage-scan-not-started = Analyse d'utilisation non démarrée
usage-files-per-category = Fichiers par catégorie
usage-filename = Nom de fichier
usage-selected-count = Sélectionné : { $count }
usage-clear-selection = Effacer la sélection
usage-configure = Configurer
usage-show-all-root-mode = Afficher tous les fichiers (mode racine)
usage-scan-setup = Configuration de l'analyse d'utilisation
usage-choose-mount-points = Choisir les points de montage
usage-choose-mount-points-desc = Sélectionnez un ou plusieurs points de montage à inclure dans l'analyse.
usage-scan-parallelism-label = Parallélisme d'analyse
usage-parallelism-low = Faible
usage-parallelism-balanced = Équilibré
usage-parallelism-high = Élevé
usage-selected = Sélectionné
usage-not-selected = Non sélectionné
usage-loading-mount-points = Chargement des points de montage...
usage-no-mount-points = Aucun point de montage disponible.
usage-parallelism = Parallélisme
usage-start-scan = Démarrer l'analyse
usage-select-at-least-one-mount-point = Sélectionnez au moins un point de montage
usage-delete-summary = Fichiers supprimés : { $deleted } ; échec de la suppression : { $failed }

# Catégories d'utilisation
usage-category-documents = Documents
usage-category-images = Images
usage-category-audio = Audio
usage-category-video = Vidéo
usage-category-archives = Archives
usage-category-code = Code
usage-category-binaries = Binaires
usage-category-packages = Paquets
usage-category-system = Système
usage-category-other = Autre
