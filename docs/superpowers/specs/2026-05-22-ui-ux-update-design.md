# UI/UX Update Design Spec

## Architecture & Components

### 1. Main Page Cleanup
- **Goal:** Provide a distraction-free experience focused on discovering and joining servers.
- **Changes:**
  - Remove "Network snapshot", "Selected server", "Diagnostics", and "Active session" panels from the `page-home` section.
  - Leave only the Hero section (with "Create host" button), the Server List, a new Search bar, and a Filter button.
  - Relocate the removed sections into the Settings page or a dedicated "Host Dashboard" that only appears when a user is actively hosting.

### 2. Host Creation Enhancements
- **Goal:** Improve UX by preventing spam clicks, adding theme selection, and speeding up port detection.
- **Changes:**
  - **Auto Port:** Perform auto-port detection in the background when the app starts or when the modal opens, so the value is pre-populated.
  - **Button State:** When "Запустить хост" is clicked, disable the button, change its text to "Создание...", and show a loading spinner.
  - **Theme Selection:** Add a grid of selectable themes (e.g., Anarchy, Roleplay, Survival, Mini-games) to the modal, mapping to a new `theme` property in the server metadata.
  - **Max Players:** Streamline player count fetching directly from presence metadata, updating instantly without staggered loading.

### 3. Server List Re-design (Ice Cube style)
- **Goal:** Create an aesthetically pleasing, detailed list of available servers.
- **Changes:**
  - **Color Codes Parser:** Implement a utility to parse Minecraft color codes (`§a`, `§l`, etc.) into styled HTML `<span>` elements for server names.
  - **Card Content:** Display ping, server version, theme, global/local type, and current online players directly on the card.
  - **Search & Filtering:**
    - Add a text input for searching by name.
    - Add a modal for filtering by Version, Theme, and Type.
  - **Animations:** Implement CSS transitions for card hover states, modal popups, and list updates.

### 4. Settings Tree Re-organization
- **Goal:** Neatly organize all non-primary functions.
- **Changes:**
  - Transform the Settings page into a tabbed or tree-based interface with sections:
    - **Account** (Login/Profile)
    - **Interface** (Theme, Accent Color, Language)
    - **Network** (Network Snapshot, Ignored Ports)
    - **Diagnostics** (Logs, Test Server)

## Data Flow
- **Search & Filter:** Kept in local state in `main.js`. Re-renders the server list DOM based on the filtered subset of Ably presence members.
- **Theme Metadata:** The host creation form appends a `theme` field to the presence data when publishing to Ably.

## Error Handling & Testing
- If background port detection fails, fallback to default (25565).
- Ensure color parser escapes HTML to prevent XSS.
