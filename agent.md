# Velocitext — agent.md

## 0) Résumé (TL;DR)
Velocitext est une application **desktop** (Tauri + Svelte) de prise de notes **local-first**.
Les notes sont des fichiers **Markdown (.md)** synchronisés entre appareils via **Syncthing** (notamment Android avec **Markor**).
L’application maintient un **index SQLite local** (non synchronisé) avec **FTS5** pour une recherche ultra rapide.
Objectif : **simple, légère, robuste**, sans cloud.

---

## 1) Stack & choix techniques

### 1.1 Frontend
- **Tauri**
- **Svelte**
- Thème **clair + sombre** (auto selon système)
- Éditeur : **Markdown brut** (pas de preview)
- Police éditeur : **monospace**
- Taille police : **14px**
- Mode **Focus/Zen** (bouton UI)

### 1.2 Backend
- Backend Tauri en **Rust**
- Accès filesystem (lecture/écriture notes)
- Watch filesystem + scan de sécurité
- Indexation SQLite + FTS5

### 1.3 Synchronisation
- **Syncthing** est la solution de sync.
- Android : édition via **Markor**.
- Aucune sync “cloud” intégrée dans Velocitext.
- L’index SQLite **ne doit jamais** être synchronisé.

---

## 2) Stockage des notes

### 2.1 Dossier des notes
- Valeur par défaut : `~/Notes/Velocitext/`
- **Configurable** par l’utilisateur (au premier lancement et ensuite)
- Le dossier est pensé pour être synchronisé par Syncthing

### 2.2 Format des fichiers
- 1 fichier `.md` = 1 note
- Encodage : UTF-8
- Extension : `.md`

### 2.3 Nommage des fichiers
Format retenu :
- `DD-MM-YYYY_HH-mm--<slug_titre>.md`

Exemple :
- `17-01-2026_14-32--café_du_matin.md`

#### Slugification
- Conserver les accents
- Espaces → `_`
- Caractères interdits (`/\:*?"<>|`) → remplacés par `-`
- Longueur max du slug du titre : **101 caractères**
- En cas de collision : suffixe `_2`, `_3`, etc.

Exemples :
- `17-01-2026_14-32--une_note_très_longue.md`
- `17-01-2026_14-32--une_note_très_longue_2.md`

### 2.4 Renommage automatique
- Si `title` change dans le frontmatter, Velocitext **renomme automatiquement** le fichier pour refléter le nouveau titre.

---

## 3) Frontmatter YAML

### 3.1 Format exact attendu
Velocitext utilise un frontmatter strict, au format :

```yaml
---
title: "un canard"
layout: note.njk
date: "17-01-2026"
description: "canards"
statut: chantier
tags:
  - canard
updated: "17-01-2026 14:32"
archived: false
---
```

Notes :
- Les valeurs ci-dessus sont des exemples.
- `layout` est toujours `note.njk` (valeur fixe).
- `date` = date de création (format `DD-MM-YYYY`)
- `updated` = dernière modification (format `DD-MM-YYYY HH:mm`)
- `statut` ∈ `idee | chantier | termine`
- `tags` est une liste YAML
- `archived` est un booléen (`true/false`)

### 3.2 Mise à jour du champ `updated`
Règle hybride :
- Velocitext met `updated` à jour automatiquement **si** le contenu a réellement changé.
- Si l’utilisateur modifie `updated` manuellement, Velocitext **respecte** la valeur.

### 3.3 Édition du YAML
- Pas de panneau “Propriétés” dédié.
- Tout est édité directement dans l’éditeur texte.

---

## 4) Indexation & recherche

### 4.1 Source de vérité
- Les fichiers `.md` sont la vérité.
- SQLite est un **index local** reconstruisible.

### 4.2 SQLite
- Emplacement recommandé :
  - `~/.local/share/velocitext/velocitext.db`
- Ne jamais stocker la DB dans le dossier synchronisé Syncthing.

### 4.3 Recherche (FTS5)
- Utiliser SQLite **FTS5** pour recherche plein texte.
- La recherche doit couvrir **tout** :
  - frontmatter complet (title, description, statut, tags, date, updated, archived, layout)
  - contenu Markdown (body)

---

## 5) Détection des changements (Syncthing-friendly)

### 5.1 Stratégie
- **Watch filesystem** (événements) pour réactivité
- + **scan de sécurité** (au démarrage + périodique) pour robustesse

### 5.2 Conflits Syncthing
- Détecter les fichiers `*sync-conflict*`
- Les afficher dans une section dédiée **“Conflits”**
- Afficher un warning clair (sans polluer la liste normale)

---

## 6) UI / UX

### 6.1 Layout principal
- Liste des notes à gauche
- Éditeur à droite

### 6.2 Création de note
- Deux méthodes :
  - Bouton “+ Nouvelle note”
  - Champ rapide “Créer une note…”
- Titre **obligatoire**

### 6.3 Tri des notes
L’utilisateur peut choisir :
- Tri par date de création (défaut)
- Tri par date de modification
- Tri par titre

### 6.4 Filtre par statut
- Contrôle discret via menu déroulant dans la barre de recherche
- Valeurs : All / idee / chantier / termine

### 6.5 Tags
- UI : chips + autocomplétion
- Stockage : YAML `tags:`

### 6.6 Archivage (soft delete)
- “Supprimer” = `archived: true`
- Les notes archivées apparaissent dans une section **Archives**
- Les fichiers ne sont pas déplacés sur disque

---

## 7) Sauvegarde

- Auto-save + Ctrl+S (force save)
- `updated` change seulement si contenu réellement modifié (diff)

---

## 8) Import / validation

### 8.1 Import
- Velocitext peut indexer un dossier existant de `.md`
- Condition : frontmatter doit correspondre **exactement** au format attendu

### 8.2 Fichiers invalides
- Les fichiers avec frontmatter invalide/incomplet sont listés dans une section :
  - **“À corriger”**
- Afficher l’erreur explicitement (champ manquant, YAML invalide, etc.)

---

## 9) Diagnostics & logs

### 9.1 Logs
- Activer une journalisation locale
- Stockage recommandé :
  - `~/.local/state/velocitext/log.txt`
- Rotation conseillée (taille max + backups)

### 9.2 Page Diagnostics (dans l’app)
Doit permettre :
- Voir les logs
- Copier les logs
- Infos : dossier notes, nombre de notes, état index, dernier scan
- Bouton “Rebuild index”
- Liste des erreurs “À corriger”

---

## 10) Packaging

### 10.1 Linux
- Priorité : **AppImage**
- Nom de fichier :
  - `velocitext_1.0.0_amd64.AppImage`

### 10.2 Icônes
- Source : SVG
- Export : PNG 1024×1024

---

## 11) Licence & repo

- Licence : **MIT**
- Nom du repo : **velocitext**
- Bundle identifier : **dev.jndjs.velocitext**
- Nom affiché : **Velocitext**

---

## 12) Architecture (modulaire)

Recommandation de structure :

- `src/` (Svelte UI)
  - `components/`
  - `routes/` (si nécessaire)
  - `stores/`
  - `lib/`
- `src-tauri/` (Rust)
  - `commands/` (API Tauri exposée au frontend)
  - `services/`
    - `notes_fs.rs`
    - `indexer.rs`
    - `search.rs`
    - `watcher.rs`
    - `diagnostics.rs`
  - `db/`
    - migrations
  - `models/`

Principes :
- Le frontend ne touche pas directement le FS.
- Le backend Rust est l’unique point d’accès au disque.
- L’index est reconstruisible.
- Les erreurs doivent être explicites et actionnables.

---

## 13) Définition “Done” (v1)
La v1 est considérée prête quand :
- Création/édition/sauvegarde de notes OK
- Sync Syncthing fonctionne sans surprise
- Index FTS5 OK + recherche rapide
- Sections : Notes / Archives / Conflits / À corriger / Diagnostics
- AppImage build OK
- Aucun crash sur dossier de notes réel

