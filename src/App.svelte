<script>
  import { onMount } from "svelte";

  let notes = [];
  let activeNote = null;
  let editorContent = "";
  let searchQuery = "";
  let searchDraft = "";
  let selectedIndex = null;
  let isProgrammaticInput = false;
  let filterStatus = "All";
  let tagFilter = null;
  let invokeFn = null;
  let searchTimeout;
  let autosaveTimeout;
  let showFrontmatter = false;
  let frontmatterBlock = "";
  let bodyContent = "";
  let lastSavedContent = "";
  let autosaveEnabled = true;
  let autosaveDelayMs = 5000;
  let focusMode = false;
  let showArchives = false;
  let draftStatut = "idee";
  let showHelp = false;
  let showSettings = false;
  let language = "fr";
  const MIN_SEARCH_CHARS = 5;

  const translations = {
    fr: {
      appTitle: "Velocitext",
      save: "Sauvegarder",
      searchPlaceholder: "Créer ou rechercher…",
      notes: "Notes",
      archives: "Archives",
      noNotes: "Aucune note",
      noArchives: "Aucune archive",
      noTags: "Aucun tag",
      showFrontmatter: "Afficher frontmatter",
      hideFrontmatter: "Masquer frontmatter",
      focus: "Focus",
      exitFocus: "Quitter focus",
      archive: "Archiver",
      restore: "Restaurer",
      statusFilterLabel: "Filtre statut",
      statusAll: "All",
      statusIdee: "Idée",
      statusChantier: "En chantier",
      statusTermine: "Terminé",
      helpTitle: "Aide",
      helpIntro:
        "Tapez le titre d'une note dans la zone de saisie pour créer ou rechercher.",
      helpSteps:
        "Appuyez sur Entrée pour créer une note si aucun résultat n'apparaît. Les flèches haut/bas naviguent dans la liste.",
      helpTags:
        "Ajoutez des tags en écrivant #exemple dans le texte : ils seront ajoutés au frontmatter.",
      helpArchive:
        "Utilisez Archiver/Restaurer pour masquer ou réafficher une note.",
      settingsTitle: "Réglages",
      settingsLanguage: "Langue de l'interface",
      langFr: "Français",
      langEn: "Anglais",
      close: "Fermer",
    },
    en: {
      appTitle: "Velocitext",
      save: "Save",
      searchPlaceholder: "Create or search…",
      notes: "Notes",
      archives: "Archives",
      noNotes: "No notes",
      noArchives: "No archives",
      noTags: "No tags",
      showFrontmatter: "Show frontmatter",
      hideFrontmatter: "Hide frontmatter",
      focus: "Focus",
      exitFocus: "Exit focus",
      archive: "Archive",
      restore: "Restore",
      statusFilterLabel: "Status filter",
      statusAll: "All",
      statusIdee: "Idea",
      statusChantier: "In progress",
      statusTermine: "Done",
      helpTitle: "Help",
      helpIntro:
        "Type a note title in the input to create or search notes.",
      helpSteps:
        "Press Enter to create when there are no results. Use up/down arrows to navigate the list.",
      helpTags:
        "Add tags by writing #example in the text; they will be added to frontmatter.",
      helpArchive:
        "Use Archive/Restore to hide or bring back a note.",
      settingsTitle: "Settings",
      settingsLanguage: "Interface language",
      langFr: "French",
      langEn: "English",
      close: "Close",
    },
  };

  const t = (key) => translations[language][key] ?? key;

  const mockNotes = [
    {
      id: "17-01-2026_14-32--cafe_du_matin.md",
      title: "Café du matin",
      layout: "note.njk",
      date: "17-01-2026",
      description: "",
      statut: "chantier",
      tags: ["café"],
      updated: "17-01-2026 14:32",
      archived: false,
    },
    {
      id: "17-01-2026_09-10--idees_rapides.md",
      title: "Idées rapides",
      layout: "note.njk",
      date: "17-01-2026",
      description: "",
      statut: "idee",
      tags: [],
      updated: "17-01-2026 09:10",
      archived: false,
    },
  ];

  const mockContent = `---
title: "Café du matin"
layout: note.njk
date: "17-01-2026"
description: ""
statut: chantier
tags:
  - café
updated: "17-01-2026 14:32"
archived: false
---

# Matin

- Exemple de contenu markdown.`;

  $: normalizedTagFilter = tagFilter ? tagFilter.toLowerCase() : null;

  $: visibleNotes = notes.filter(
    (note) =>
      !note.archived &&
      (filterStatus === "All" || note.statut === filterStatus) &&
      (!normalizedTagFilter ||
        (note.tags ?? []).some(
          (tag) => tag.toLowerCase() === normalizedTagFilter
        ))
  );

  $: archivedNotes = notes.filter(
    (note) =>
      note.archived &&
      (filterStatus === "All" || note.statut === filterStatus) &&
      (!normalizedTagFilter ||
        (note.tags ?? []).some(
          (tag) => tag.toLowerCase() === normalizedTagFilter
        ))
  );

  $: allTags = Array.from(
    new Set(
      notes
        .flatMap((note) => note.tags ?? [])
        .map((tag) => tag.trim())
        .filter(Boolean)
    )
  ).sort((left, right) => left.localeCompare(right, "fr", { sensitivity: "base" }));

  $: if (selectedIndex !== null && selectedIndex >= visibleNotes.length) {
    selectedIndex = null;
  }

  $: activeTitle = activeNote ? activeNote.metadata.title : "";

  async function ensureInvoke() {
    if (invokeFn || !window.__TAURI__) {
      return;
    }

    const api = await import("@tauri-apps/api/tauri");
    invokeFn = api.invoke;
  }

  function extractTitleAndBody(content) {
    const lines = content.split(/\r?\n/);
    const title = (lines[0] ?? "").trim();
    let body = lines.slice(1).join("\n");
    if (body.startsWith("\n")) {
      body = body.slice(1);
    }
    return { title, body };
  }

  function parseNoteContent(content) {
    const lines = content.split(/\r?\n/);
    if (!lines[0] || lines[0].trim() !== "---") {
      return { frontmatter: "", body: content };
    }

    let endIndex = -1;
    for (let i = 1; i < lines.length; i += 1) {
      if (lines[i].trim() === "---") {
        endIndex = i;
        break;
      }
    }

    if (endIndex === -1) {
      return { frontmatter: "", body: content };
    }

    const frontmatter = lines.slice(0, endIndex + 1).join("\n");
    let body = lines.slice(endIndex + 1).join("\n");
    if (body.startsWith("\n")) {
      body = body.slice(1);
    }
    return { frontmatter, body };
  }

  function buildRaw(frontmatter, body) {
    if (!frontmatter) {
      return body;
    }
    if (!body) {
      return `${frontmatter}\n`;
    }
    return `${frontmatter}\n\n${body}`;
  }

  function buildEditorBody(frontmatter, body, titleOverride = "") {
    if (!frontmatter && !titleOverride) {
      return body;
    }
    const titleMatch = frontmatter?.match(/^title:\s*"(.*)"$/m);
    const title = titleOverride || (titleMatch ? titleMatch[1] : "");
    if (!body) {
      return title;
    }
    return `${title}\n${body}`;
  }

  function updateStatutLine(content, statut) {
    if (!content) {
      return content;
    }
    return content.replace(/^statut:\s*.*$/m, `statut: ${statut}`);
  }

  function updateTitleLine(content, title) {
    if (!content) {
      return content;
    }
    return content.replace(/^title:\s*".*"$/m, `title: "${escapeYamlValue(title)}"`);
  }

  function escapeYamlValue(value) {
    return value.replace(/"/g, "\\\"");
  }

  function parseTagsFromFrontmatter(frontmatter) {
    if (!frontmatter) {
      return [];
    }

    const lines = frontmatter.split(/\r?\n/);
    const tagsIndex = lines.findIndex((line) => line.trim().startsWith("tags:"));
    if (tagsIndex === -1) {
      return [];
    }

    const tagsLine = lines[tagsIndex].trim();
    if (tagsLine === "tags: []") {
      return [];
    }

    const tags = [];
    for (let i = tagsIndex + 1; i < lines.length; i += 1) {
      const line = lines[i];
      if (!line.trim().startsWith("- ") && !line.trim().startsWith("-")) {
        break;
      }
      const raw = line.replace(/^\s*-\s*/, "").trim();
      const cleaned = raw.replace(/^"|"$/g, "").trim();
      if (cleaned) {
        tags.push(cleaned);
      }
    }

    return tags;
  }

  function buildTagsBlock(tags) {
    if (!tags.length) {
      return ["tags: []"];
    }

    return [
      "tags:",
      ...tags.map((tag) => `  - ${escapeYamlValue(tag)}`),
    ];
  }

  function updateTagsBlock(frontmatter, tags) {
    if (!frontmatter) {
      return frontmatter;
    }

    const lines = frontmatter.split(/\r?\n/);
    const tagsIndex = lines.findIndex((line) => line.trim().startsWith("tags:"));
    if (tagsIndex === -1) {
      return frontmatter;
    }

    let endIndex = tagsIndex + 1;
    while (endIndex < lines.length && lines[endIndex].trim().startsWith("-")) {
      endIndex += 1;
    }

    const updated = [
      ...lines.slice(0, tagsIndex),
      ...buildTagsBlock(tags),
      ...lines.slice(endIndex),
    ];

    return updated.join("\n");
  }

  function extractHashtags(content) {
    if (!content) {
      return [];
    }

    const matches = content.match(/(^|\s)#([^\s#]+)/g) ?? [];
    const tags = matches
      .map((match) => match.replace(/^\s*#/, ""))
      .map((tag) => tag.replace(/[\s.,;:!?]+$/g, "").trim())
      .filter(Boolean);

    return Array.from(new Set(tags));
  }

  function mergeTags(existingTags, newTags) {
    const merged = [...existingTags];
    newTags.forEach((tag) => {
      if (!merged.some((existing) => existing.toLowerCase() === tag.toLowerCase())) {
        merged.push(tag);
      }
    });
    return merged;
  }

  function contentForSave() {
    if (showFrontmatter) {
      return editorContent;
    }

    const { title, body } = extractTitleAndBody(editorContent);
    if (!frontmatterBlock) {
      return body;
    }

    const updatedFrontmatter = updateTitleLine(frontmatterBlock, title);
    return buildRaw(updatedFrontmatter, body);
  }

  function setEditorContent(content, titleOverride = "") {
    const parsed = parseNoteContent(content);
    frontmatterBlock = parsed.frontmatter;
    bodyContent = parsed.body;
    editorContent = showFrontmatter
      ? buildRaw(frontmatterBlock, bodyContent)
      : buildEditorBody(frontmatterBlock, bodyContent, titleOverride);
    lastSavedContent = content;
  }

  function toggleFrontmatter() {
    if (!activeNote) {
      return;
    }

    if (showFrontmatter) {
      const parsed = parseNoteContent(editorContent);
      if (parsed.frontmatter) {
        frontmatterBlock = parsed.frontmatter;
      }
      bodyContent = parsed.body;
      editorContent = buildEditorBody(frontmatterBlock, bodyContent);
      showFrontmatter = false;
    } else {
      const { title, body } = extractTitleAndBody(editorContent);
      bodyContent = body;
      if (frontmatterBlock) {
        frontmatterBlock = updateTitleLine(frontmatterBlock, title);
      }
      editorContent = buildRaw(frontmatterBlock, bodyContent);
      showFrontmatter = true;
    }
  }

  function toggleFocusMode() {
    focusMode = !focusMode;
  }

  function setSearchQuery(value, { programmatic = false } = {}) {
    if (programmatic) {
      isProgrammaticInput = true;
    }
    searchQuery = value;
  }

  function clearSelection() {
    selectedIndex = null;
    activeNote = null;
    frontmatterBlock = "";
    bodyContent = "";
    lastSavedContent = "";
    draftStatut = "idee";
    setSearchQuery(searchDraft, { programmatic: true });
  }

  function handleSearchInput(event) {
    if (isProgrammaticInput) {
      isProgrammaticInput = false;
      return;
    }

    searchDraft = event.target.value;
    searchQuery = searchDraft;
    selectedIndex = null;
    if (activeNote) {
      activeNote = null;
      frontmatterBlock = "";
      bodyContent = "";
      lastSavedContent = "";
    }
    if (searchDraft.trim().length >= MIN_SEARCH_CHARS) {
      scheduleSearch();
    } else {
      loadNotes();
    }
  }

  function handleSearchKeydown(event) {
    if (event.key === "Enter") {
      event.preventDefault();
      const title = searchDraft.trim();
      if (!title) {
        return;
      }
      if (visibleNotes.length === 0) {
        createNote(title);
        return;
      }
      const index = selectedIndex ?? 0;
      selectNoteByIndex(index);
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      moveSelection(1);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      moveSelection(-1);
      return;
    }

    if (event.key === "Escape" && selectedIndex !== null) {
      event.preventDefault();
      clearSelection();
    }
  }

  function moveSelection(delta) {
    if (!visibleNotes.length) {
      return;
    }

    let nextIndex = selectedIndex;
    if (nextIndex === null) {
      nextIndex = delta > 0 ? 0 : visibleNotes.length - 1;
    } else {
      nextIndex = Math.max(0, Math.min(visibleNotes.length - 1, nextIndex + delta));
    }

    selectNoteByIndex(nextIndex);
  }

  function handleEditorInput() {
    if (activeNote) {
      scheduleAutosave();
      return;
    }

    const { title } = extractTitleAndBody(editorContent);
    searchDraft = title;
    setSearchQuery(title, { programmatic: true });
    selectedIndex = null;
    if (title.trim().length >= MIN_SEARCH_CHARS) {
      scheduleSearch();
    } else {
      loadNotes();
    }
  }

  function handleStatutChange(event) {
    draftStatut = event.target.value;
    if (activeNote) {
      if (showFrontmatter) {
        editorContent = updateStatutLine(editorContent, draftStatut);
        const parsed = parseNoteContent(editorContent);
        frontmatterBlock = parsed.frontmatter;
        bodyContent = parsed.body;
      } else if (frontmatterBlock) {
        frontmatterBlock = updateStatutLine(frontmatterBlock, draftStatut);
      }

      activeNote = {
        ...activeNote,
        metadata: {
          ...activeNote.metadata,
          statut: draftStatut,
        },
      };

      scheduleAutosave();
    }
  }

  function toggleTagFilter(tag) {
    tagFilter = tagFilter === tag ? null : tag;
    selectedIndex = null;
    activeNote = null;
    frontmatterBlock = "";
    bodyContent = "";
    lastSavedContent = "";
  }

  function scheduleAutosave() {
    if (!autosaveEnabled || !activeNote || !invokeFn) {
      return;
    }

    const content = contentForSave();
    if (content === lastSavedContent) {
      return;
    }

    clearTimeout(autosaveTimeout);
    autosaveTimeout = setTimeout(() => {
      saveNote();
    }, autosaveDelayMs);
  }

  async function loadNotes() {
    if (!invokeFn) {
      return;
    }

    notes = await invokeFn("list_notes");
  }

  async function selectNoteByIndex(index) {
    const note = visibleNotes[index];
    if (!note) {
      return;
    }

    selectedIndex = index;
    await selectNote(note, true);
  }

  async function selectNote(note, updateInput = false) {
    if (!invokeFn) {
      activeNote = { metadata: note, content: mockContent };
      draftStatut = note.statut ?? "idee";
      setEditorContent(mockContent, note.title);
      if (updateInput) {
        setSearchQuery(note.title, { programmatic: true });
      }
      return;
    }

    const detail = await invokeFn("get_note", { id: note.id });
    activeNote = detail;
    draftStatut = detail.metadata.statut ?? "idee";
    setEditorContent(detail.content, detail.metadata.title);
    if (updateInput) {
      setSearchQuery(note.title, { programmatic: true });
    }
  }

  async function createNote(titleOverride) {
    if (!invokeFn) {
      return;
    }

    const title = titleOverride ?? window.prompt("Titre de la note");
    if (!title) {
      return;
    }

    const detail = await invokeFn("create_note", { title });
    await loadNotes();
    activeNote = detail;
    draftStatut = detail.metadata.statut ?? "idee";
    setEditorContent(detail.content, detail.metadata.title);
    searchDraft = title;
    setSearchQuery(title, { programmatic: true });
    const index = visibleNotes.findIndex((note) => note.id === detail.metadata.id);
    selectedIndex = index === -1 ? null : index;
  }

  async function saveNote() {
    if (!invokeFn) {
      return;
    }

    if (!activeNote) {
      await saveDraftNote();
      return;
    }

    let contentToSave = contentForSave();
    let frontmatter = frontmatterBlock;
    let bodyForTags = editorContent;

    if (showFrontmatter) {
      const parsed = parseNoteContent(editorContent);
      frontmatter = parsed.frontmatter;
      bodyForTags = parsed.body;
      contentToSave = editorContent;
    } else {
      const { title, body } = extractTitleAndBody(editorContent);
      bodyForTags = body;
      if (frontmatter) {
        frontmatter = updateTitleLine(frontmatter, title);
      }
    }

    if (draftStatut && frontmatter) {
      frontmatter = updateStatutLine(frontmatter, draftStatut);
    }

    if (frontmatter) {
      const existingTags = parseTagsFromFrontmatter(frontmatter);
      const newTags = extractHashtags(bodyForTags);
      const mergedTags = mergeTags(existingTags, newTags);
      frontmatter = updateTagsBlock(frontmatter, mergedTags);
    }

    contentToSave = frontmatter
      ? showFrontmatter
        ? buildRaw(frontmatter, bodyForTags)
        : buildRaw(frontmatter, bodyForTags)
      : contentToSave;

    const detail = await invokeFn("save_note", {
      id: activeNote.metadata.id,
      content: contentToSave,
    });
    await loadNotes();
    activeNote = detail;
    setEditorContent(detail.content, detail.metadata.title);
    lastSavedContent = contentToSave;
  }

  async function saveDraftNote() {
    const { title, body } = extractTitleAndBody(editorContent);
    if (!title.trim()) {
      return;
    }

    const matchIndex = visibleNotes.findIndex(
      (note) => note.title.toLowerCase() === title.toLowerCase()
    );
    if (matchIndex !== -1) {
      await selectNoteByIndex(matchIndex);
      return;
    }

    const detail = await invokeFn("create_note", { title });
    let contentToSave = detail.content;

    if (draftStatut && draftStatut !== "idee") {
      contentToSave = updateStatutLine(contentToSave, draftStatut);
    }

    const existingTags = parseTagsFromFrontmatter(contentToSave);
    const newTags = extractHashtags(body);
    const mergedTags = mergeTags(existingTags, newTags);
    contentToSave = updateTagsBlock(contentToSave, mergedTags);

    contentToSave = body ? `${contentToSave}\n${body}` : contentToSave;

    const saved = await invokeFn("save_note", {
      id: detail.metadata.id,
      content: contentToSave,
    });
    await loadNotes();
    activeNote = saved;
    draftStatut = saved.metadata.statut ?? draftStatut;
    setEditorContent(saved.content, saved.metadata.title);
    searchDraft = title;
    setSearchQuery(title, { programmatic: true });
    const index = visibleNotes.findIndex((note) => note.id === saved.metadata.id);
    selectedIndex = index === -1 ? null : index;
  }

  async function toggleArchive() {
    if (!invokeFn || !activeNote) {
      return;
    }

    const target = !activeNote.metadata.archived;
    const detail = await invokeFn("archive_note", {
      id: activeNote.metadata.id,
      archived: target,
    });
    await loadNotes();
    activeNote = detail;
    setEditorContent(detail.content, detail.metadata.title);
  }

  function scheduleSearch() {
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(runSearch, 200);
  }

  function findPrefixMatch(list, query) {
    const normalized = query.toLowerCase();
    return list.findIndex((note) => note.title.toLowerCase().startsWith(normalized));
  }

  async function runSearch() {
    if (!invokeFn) {
      return;
    }

    const query = searchDraft.trim();
    if (!query || query.length < MIN_SEARCH_CHARS) {
      await loadNotes();
      return;
    }

    notes = await invokeFn("search_notes", { query });

    if (selectedIndex === null && notes.length > 0) {
      const matchIndex = findPrefixMatch(visibleNotes, query);
      if (matchIndex !== -1) {
        await selectNoteByIndex(matchIndex);
        setSearchQuery(visibleNotes[matchIndex].title, { programmatic: true });
      }
    }
  }

  onMount(async () => {
    if (window.__TAURI__) {
      await ensureInvoke();
      await loadNotes();

      const { listen } = await import("@tauri-apps/api/event");
      await listen("menu://help", () => {
        showHelp = true;
      });
      await listen("menu://settings", () => {
        showSettings = true;
      });
    } else {
      notes = mockNotes;
      activeNote = null;
      editorContent = "";
    }
  });
</script>

<main class:focus={focusMode} class="app">
  <aside class="sidebar">
    <header class="sidebar__header">
      <div class="app-title">
        <img class="app-icon" src="/icon.png" alt="" aria-hidden="true" />
        <h1>{t("appTitle")}</h1>
      </div>
    </header>

    <div class="search">
      <input
        placeholder={t("searchPlaceholder")}
        type="search"
        bind:value={searchQuery}
        on:input={handleSearchInput}
        on:keydown={handleSearchKeydown}
      />
      <div class="status-filter" role="group" aria-label={t("statusFilterLabel")}>
        <label>
          <input
            type="radio"
            name="status-filter"
            value="All"
            checked={filterStatus === "All"}
            on:change={() => (filterStatus = "All")}
          />
          {t("statusAll")}
        </label>
        <label>
          <input
            type="radio"
            name="status-filter"
            value="idee"
            checked={filterStatus === "idee"}
            on:change={() => (filterStatus = "idee")}
          />
          {t("statusIdee")}
        </label>
        <label>
          <input
            type="radio"
            name="status-filter"
            value="chantier"
            checked={filterStatus === "chantier"}
            on:change={() => (filterStatus = "chantier")}
          />
          {t("statusChantier")}
        </label>
        <label>
          <input
            type="radio"
            name="status-filter"
            value="termine"
            checked={filterStatus === "termine"}
            on:change={() => (filterStatus = "termine")}
          />
          {t("statusTermine")}
        </label>
      </div>
    </div>

    <div class="tags">
      {#if allTags.length === 0}
        <span class="tag empty">{t("noTags")}</span>
      {:else}
        {#each allTags as tag}
          <button
            type="button"
            class:active={tagFilter === tag}
            class="tag"
            on:click={() => toggleTagFilter(tag)}
          >
            #{tag}
          </button>
        {/each}
      {/if}
    </div>

    <div class="notes-section">
      <h3>{t("notes")}</h3>
      <ul class="notes">
        {#if visibleNotes.length === 0}
          <li class="note empty">{t("noNotes")}</li>
        {:else}
          {#each visibleNotes as note, index}
            <li class="note-item">
              <button
                class:active={activeNote && activeNote.metadata.id === note.id}
                class="note"
                type="button"
                on:click={() => selectNoteByIndex(index)}
              >
                <h2>{note.title}</h2>
                <p>{note.date} · {note.statut}</p>
              </button>
            </li>
          {/each}
        {/if}
      </ul>
    </div>

    <div class="notes-section">
      <div class="section-header">
        <h3>{t("archives")}</h3>
        <button class="toggle" type="button" on:click={() => (showArchives = !showArchives)}>
          {showArchives ? "–" : "+"}
        </button>
      </div>
      {#if showArchives}
        <ul class="notes">
          {#if archivedNotes.length === 0}
            <li class="note empty">{t("noArchives")}</li>
          {:else}
            {#each archivedNotes as note}
              <li class="note-item">
                <button
                  class:active={activeNote && activeNote.metadata.id === note.id}
                  class="note"
                  type="button"
                  on:click={() => selectNote(note, true)}
                >
                  <h2>{note.title}</h2>
                  <p>{note.date} · {note.statut}</p>
                </button>
              </li>
            {/each}
          {/if}
        </ul>
      {/if}
    </div>
  </aside>

  <section class="editor">
    <header class="editor__header">
      <input
        class="title"
        value={activeTitle}
        placeholder={t("notes")}
        readonly
      />
      <div class="editor__actions">
        <button disabled={!activeNote} on:click={toggleFrontmatter}>
          {showFrontmatter ? t("hideFrontmatter") : t("showFrontmatter")}
        </button>
        <button disabled={!activeNote} on:click={toggleFocusMode}>
          {focusMode ? t("exitFocus") : t("focus")}
        </button>
        <button disabled={!activeNote} on:click={toggleArchive}>
          {activeNote && activeNote.metadata.archived
            ? t("restore")
            : t("archive")}
        </button>
      </div>
    </header>
    <div class="editor__body">
      <textarea
        spellcheck="false"
        bind:value={editorContent}
        on:input={handleEditorInput}
        placeholder={t("searchPlaceholder")}
      ></textarea>
      <div class="editor__footer">
        <div class="status-group" role="group" aria-label="Statut">
          <label>
            <input
              type="radio"
              name="statut"
              value="idee"
              checked={draftStatut === "idee"}
              on:change={handleStatutChange}
            />
            {t("statusIdee")}
          </label>
          <label>
            <input
              type="radio"
              name="statut"
              value="chantier"
              checked={draftStatut === "chantier"}
              on:change={handleStatutChange}
            />
            {t("statusChantier")}
          </label>
          <label>
            <input
              type="radio"
              name="statut"
              value="termine"
              checked={draftStatut === "termine"}
              on:change={handleStatutChange}
            />
            {t("statusTermine")}
          </label>
        </div>
        <button class="primary" on:click={saveNote}>
          {t("save")}
        </button>
      </div>
    </div>
  </section>
</main>

{#if showHelp}
  <div class="modal" role="dialog" aria-modal="true">
    <div class="modal__content">
      <h2>{t("helpTitle")}</h2>
      <p>{t("helpIntro")}</p>
      <p>{t("helpSteps")}</p>
      <p>{t("helpTags")}</p>
      <p>{t("helpArchive")}</p>
      <button class="primary" on:click={() => (showHelp = false)}>
        {t("close")}
      </button>
    </div>
  </div>
{/if}

{#if showSettings}
  <div class="modal" role="dialog" aria-modal="true">
    <div class="modal__content">
      <h2>{t("settingsTitle")}</h2>
      <div class="settings-group">
        <p>{t("settingsLanguage")}</p>
        <label>
          <input
            type="radio"
            name="language"
            value="fr"
            checked={language === "fr"}
            on:change={() => (language = "fr")}
          />
          {t("langFr")}
        </label>
        <label>
          <input
            type="radio"
            name="language"
            value="en"
            checked={language === "en"}
            on:change={() => (language = "en")}
          />
          {t("langEn")}
        </label>
      </div>
      <button class="primary" on:click={() => (showSettings = false)}>
        {t("close")}
      </button>
    </div>
  </div>
{/if}
