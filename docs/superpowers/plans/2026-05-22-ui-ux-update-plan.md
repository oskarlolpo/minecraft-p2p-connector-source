# UI/UX Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Overhaul the main UI to focus exclusively on server discovery, streamline host creation with themes and background port detection, and move auxiliary tools to a tabbed settings menu.

**Architecture:** We will re-structure `index.html` to create a tabbed settings layout, add new modals for filters, and use JavaScript in `main.js` to manage the new state, Minecraft color code parsing, and background port fetching.

**Tech Stack:** HTML, CSS, Vanilla JavaScript, Tauri (Rust backend for port detection).

---

### Task 1: Re-structure `index.html` UI (Settings Tabs & Moving Panels)

**Files:**
- Modify: `src/index.html`

- [ ] **Step 1: Create the Settings tab navigation**
Modify the `<section id="page-settings" class="page">` to include a sub-navigation bar for settings categories: Account, Interface, Network, Diagnostics.

```html
<nav class="settings-tabs" style="display: flex; gap: 16px; margin-bottom: 24px; border-bottom: 1px solid var(--line); padding-bottom: 8px;">
  <button class="settings-tab active" data-tab="account">Аккаунт</button>
  <button class="settings-tab" data-tab="interface">Интерфейс</button>
  <button class="settings-tab" data-tab="network">Сеть</button>
  <button class="settings-tab" data-tab="diagnostics">Диагностика</button>
</nav>
```

- [ ] **Step 2: Wrap existing settings into tab containers**
Wrap Account panels into `<div class="settings-tab-content" id="tab-account">`, Interface (Theme/Accent/Lang) into `tab-interface`, and so on.

- [ ] **Step 3: Move side-column panels to Settings**
Move `.snapshot-panel` (Сетевой снимок), `.selected-panel` (Выбранный сервер), and the Diagnostics panel from `page-home` into their respective tabs in `page-settings` (Network and Diagnostics).

- [ ] **Step 4: Clean up `page-home`**
Ensure `page-home` only contains the `.hero-panel` and the server list area.

- [ ] **Step 5: Commit changes**
```bash
git add src/index.html
git commit -m "feat: restructure index.html for tabbed settings and clean home page"
```

### Task 2: Host Creation Updates (Auto-Port & Themes) in `index.html`

**Files:**
- Modify: `src/index.html`

- [ ] **Step 1: Add Theme Selection to Host Modal**
Inside `#host-modal`, before the external host options, add a theme selector grid.

```html
<div class="field">
  <span data-i18n="modalThemeLabel">Тематика сервера</span>
  <div class="theme-grid" id="host-theme-grid" style="display: flex; flex-wrap: wrap; gap: 8px;">
    <button type="button" class="theme-btn active" data-theme="Выживание">⛏ Выживание</button>
    <button type="button" class="theme-btn" data-theme="Анархия">💀 Анархия</button>
    <button type="button" class="theme-btn" data-theme="Мини-игры">🎮 Мини-игры</button>
    <button type="button" class="theme-btn" data-theme="РПГ">🗡 РПГ</button>
    <button type="button" class="theme-btn" data-theme="Общение">💬 Общение</button>
    <button type="button" class="theme-btn" data-theme="Другое">✨ Другое</button>
  </div>
</div>
```

- [ ] **Step 2: Add Search & Filter Controls to Home**
In `page-home`, right above `<div id="server-list">`, add a search bar and filter button.

```html
<div class="server-list-controls" style="display: flex; gap: 12px; margin-bottom: 16px;">
  <input type="text" id="server-search-input" placeholder="Поиск серверов..." style="flex: 1;" />
  <button type="button" id="open-filter-modal" class="ghost-button">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="4" y1="21" x2="4" y2="14"/><line x1="4" y1="10" x2="4" y2="3"/><line x1="12" y1="21" x2="12" y2="12"/><line x1="12" y1="8" x2="12" y2="3"/><line x1="20" y1="21" x2="20" y2="16"/><line x1="20" y1="12" x2="20" y2="3"/><line x1="1" y1="14" x2="7" y2="14"/><line x1="9" y1="8" x2="15" y2="8"/><line x1="17" y1="16" x2="23" y2="16"/></svg>
  </button>
</div>
```

- [ ] **Step 3: Commit**
```bash
git add src/index.html
git commit -m "feat: add theme selection, search, and filter controls"
```

### Task 3: CSS Styling

**Files:**
- Modify: `src/styles.css`

- [ ] **Step 1: Add styles for Settings Tabs**
```css
.settings-tab {
  background: transparent; border: none; color: var(--text-soft); font-size: 14px; padding: 8px 16px; cursor: pointer; border-radius: var(--radius-sm); transition: all 0.2s ease;
}
.settings-tab.active {
  background: var(--surface-float); color: var(--text-base); font-weight: 500;
}
.settings-tab-content { display: none; }
.settings-tab-content.active { display: block; animation: fadeIn 0.3s ease; }
@keyframes fadeIn { from { opacity: 0; transform: translateY(5px); } to { opacity: 1; transform: translateY(0); } }
```

- [ ] **Step 2: Add styles for Server Card Redesign & Themes**
```css
.theme-btn {
  background: var(--surface-raised); border: 1px solid var(--line); color: var(--text-base); padding: 6px 12px; border-radius: 16px; cursor: pointer; transition: all 0.2s; font-size: 13px;
}
.theme-btn.active {
  background: var(--accent-blue); color: white; border-color: var(--accent-blue);
}
.server-card { transition: transform 0.2s, border-color 0.2s; }
.server-card:hover { transform: translateY(-2px); border-color: var(--line-strong); }
/* Add styles for Minecraft colors */
.mc-color-a { color: #55FF55; }
.mc-color-c { color: #FF5555; }
.mc-color-e { color: #FFFF55; }
.mc-color-f { color: #FFFFFF; }
.mc-color-l { font-weight: bold; }
.mc-color-n { text-decoration: underline; }
.mc-color-o { font-style: italic; }
.mc-color-m { text-decoration: line-through; }
/* Add more codes as necessary */
```

- [ ] **Step 3: Commit**
```bash
git add src/styles.css
git commit -m "style: apply settings tabs and server card styling"
```

### Task 4: Javascript Logic (Background Port & Modals)

**Files:**
- Modify: `src/main.js`

- [ ] **Step 1: Tab switching logic**
Add an event listener to `.settings-tab` elements to switch the `.active` class on tabs and `.settings-tab-content` elements.

- [ ] **Step 2: Pre-fetch background port**
Inside the initialization phase of `main.js`, trigger the Tauri command to fetch the local game port, and save it to a variable `prefetchedAutoPort`. When the host modal opens, instantly populate `#local-game-port` with this value instead of waiting for a button click.

- [ ] **Step 3: Host button state management**
When `#host-button` is clicked, add:
```javascript
const btn = document.getElementById('host-button');
btn.disabled = true;
btn.textContent = "Создание...";
// Reset upon success or failure.
```

- [ ] **Step 4: Commit**
```bash
git add src/main.js
git commit -m "feat: background port fetching, tab switching, button state"
```

### Task 5: Minecraft Color Parser & Server Card Rendering

**Files:**
- Modify: `src/main.js`

- [ ] **Step 1: Write `parseMinecraftColors` utility**
```javascript
function parseMinecraftColors(text) {
  if (!text) return '';
  const map = {
    '0': 'color: #000000;', '1': 'color: #0000AA;', '2': 'color: #00AA00;', '3': 'color: #00AAAA;',
    '4': 'color: #AA0000;', '5': 'color: #AA00AA;', '6': 'color: #FFAA00;', '7': 'color: #AAAAAA;',
    '8': 'color: #555555;', '9': 'color: #5555FF;', 'a': 'color: #55FF55;', 'b': 'color: #55FFFF;',
    'c': 'color: #FF5555;', 'd': 'color: #FF55FF;', 'e': 'color: #FFFF55;', 'f': 'color: #FFFFFF;',
    'l': 'font-weight: bold;', 'n': 'text-decoration: underline;', 'o': 'font-style: italic;', 'm': 'text-decoration: line-through;'
  };
  
  let html = '';
  let parts = text.split('§');
  html += parts[0]; // first part has no color
  let activeStyles = [];
  
  for (let i = 1; i < parts.length; i++) {
    const code = parts[i].charAt(0).toLowerCase();
    const content = parts[i].slice(1);
    
    if (code === 'r') {
      activeStyles = [];
    } else if (map[code]) {
      // If it's a color, replace existing colors, keep formats
      if (/[0-9a-f]/.test(code)) {
         activeStyles = activeStyles.filter(s => !s.startsWith('color:'));
      }
      activeStyles.push(map[code]);
    }
    
    if (content) {
      html += `<span style="${activeStyles.join(' ')}">${content.replace(/</g, "&lt;").replace(/>/g, "&gt;")}</span>`;
    }
  }
  return html;
}
```

- [ ] **Step 2: Update Server Card rendering (`renderServerList`)**
Modify the logic that builds the server rows. Parse the MOTD/name with the color parser. Read the new `theme` metadata from the presence member and display it. Display ping dynamically.

- [ ] **Step 3: Commit**
```bash
git add src/main.js
git commit -m "feat: parse minecraft colors and update server card UI"
```

### Task 6: Search & Filter Logic

**Files:**
- Modify: `src/main.js`

- [ ] **Step 1: Search event listener**
Bind `input` event on `#server-search-input` to trigger a re-render of the server list, filtering presence members whose name or motd includes the search string.

- [ ] **Step 2: Theme Selection Logic (Host Modal)**
Bind click events on `.theme-btn` inside `#host-theme-grid` to set an active theme variable. Inject this `theme` into the Ably presence data when hosting.

- [ ] **Step 3: Commit**
```bash
git add src/main.js
git commit -m "feat: implement search and host theme metadata saving"
```
