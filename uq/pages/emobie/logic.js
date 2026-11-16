// Pure logic of the Emoji and macros page: reading `emobie status`, wording, and checking what
// the person typed before a command is run. No Qt calls, so Node can test it.
.pragma library

// Words for each setting (the CLI has keys, values and rules; the page has people's words).
var META = {
    theme: { label: "Theme", help: "", choices: [["system", "Follow the system"], ["light", "Light"], ["dark", "Dark"]] },
    showTitleBar: { label: "Show the window title bar", help: "" },
    emojiSize: { label: "Size of the emoji grid", help: "", choices: [["sm", "Small"], ["md", "Medium"], ["lg", "Large"]] },
    sortBy: { label: "Order of the emoji grid", help: "", choices: [["default", "Default"], ["name", "Name"], ["type", "Category"], ["dateAdded", "First used"], ["uses", "Most used"]] },
    skinTone: { label: "Skin tone", help: "", lowLabel: "Default", highLabel: "Dark" },
    recentMax: { label: "Recent emoji kept", help: "How many recently used emoji the palette remembers.", lowLabel: "8", highLabel: "96" },
    favoriteEmojiMacros: { label: "Add favourites as macros", help: "Shortcodes and emoticons for your favourite emoji appear under Macros in emobie." },
    emoticonStyle: { label: "Emoticon style", help: "", choices: [["minimal", ":) style"], ["classic", ":-) style"]] },
    hotkey: { label: "Summon shortcut", help: "Shows or hides emobie from anywhere. Like Control+Shift+Space; a letter needs Control, Alt or Meta." },
    launchOnStartup: { label: "Start with the desktop", help: "" },
    startMinimizedToTray: { label: "Start hidden in the tray", help: "" },
    allowMultipleInstances: { label: "Allow more than one window", help: "" },
    checkUpdatesOnStartup: { label: "Check for updates when it starts", help: "" },
    expandTriggerMode: { label: "When macros expand", help: "Used while Expand as you type is on.", choices: [["space", "After Space"], ["immediate", "As soon as typed"]] },
    expandKeepTriggerSpace: { label: "Keep the Space after an expansion", help: "" },
    expandRestoreClipboard: { label: "Put the clipboard back after pasting", help: "Off by default: restoring can wipe your next copy on some desktops." },
    pasteChordOverride: { label: "Paste key", help: "What emobie presses to paste. Auto picks by the focused app.", choices: [["auto", "Auto"], ["ctrl_v", "Ctrl+V"], ["ctrl_shift_v", "Ctrl+Shift+V"], ["shift_insert", "Shift+Insert"]] }
};

function metaOf(key) {
    return META[key] || { label: key, help: "" };
}

function isObject(v) {
    return v !== null && typeof v === "object" && !Array.isArray(v);
}

// A view with nothing in it, so every tab has something to read whatever happened.
function blank(status, error) {
    return { status: status, error: error, saved: false, version: "", deletedMax: 50, rows: {}, macros: [], deleted: [] };
}

// What the page shows from `emobie status --json`: {status: "ready" | "error", error, ...}.
function readStatus(result) {
    if (!result)
        return blank("error", "No answer from emobie.");
    const doc = result.output;
    if (!isObject(doc) || !isObject(doc.config)) {
        if (isObject(doc) && typeof doc.error === "string")
            return blank("error", doc.error);
        return blank("error", result.error ? String(result.error) : "emobie answered with something this page can't read.");
    }
    const rows = {};
    for (const key in doc.config) {
        const r = doc.config[key];
        if (isObject(r))
            rows[key] = r;
    }
    const app = isObject(doc.app) ? doc.app : {};
    const view = blank("ready", "");
    view.saved = app.saved === true;
    view.version = typeof app.version === "string" ? app.version : "";
    view.deletedMax = typeof app.deleted_max === "number" ? app.deleted_max : 50;
    view.rows = rows;
    view.macros = Array.isArray(doc.macros) ? doc.macros.filter(isObject) : [];
    view.deleted = Array.isArray(doc.deleted) ? doc.deleted.filter(isObject) : [];
    return view;
}

function row(view, key) {
    const r = view && view.rows ? view.rows[key] : null;
    return r || { value: null, default: null, type: "", readonly: false, saved: false };
}

function isCancelled(result) {
    return !!(result && result.cancelled);
}

// A write's answer: emobie prints {ok, error} even when it refuses.
function envelope(result) {
    if (!result)
        return { ok: false, error: "No answer from emobie." };
    if (isObject(result.output) && result.output.ok === true)
        return { ok: true, error: "", summary: String(result.output.summary || "") };
    if (isObject(result.output) && typeof result.output.error === "string")
        return { ok: false, error: result.output.error };
    if (result.error)
        return { ok: false, error: String(result.error).slice(0, 300) };
    return { ok: result.code === 0, error: result.code === 0 ? "" : "emobie couldn't do that." };
}

function listText(list) {
    return Array.isArray(list) ? list.join(", ") : "";
}

function oneLine(text, max) {
    const s = String(text === null || text === undefined ? "" : text).replace(/[\r\n\t]+/g, " ");
    return s.length > max ? s.slice(0, max - 1) + "…" : s;
}

// The names of the macros shortcuts: "Control+Alt+S" -> "Ctrl+Alt+S".
function hotkeyText(h) {
    return typeof h === "string" ? h.replace(/^Control\+/, "Ctrl+").replace(/\+Control\+/g, "+Ctrl+") : "";
}

function whenText(ms, now) {
    if (typeof ms !== "number" || ms <= 0)
        return "";
    const secs = Math.max(0, Math.floor((now - ms) / 1000));
    if (secs < 60)
        return "just now";
    if (secs < 3600)
        return Math.floor(secs / 60) + " min ago";
    if (secs < 86400) {
        const h = Math.floor(secs / 3600);
        return h + (h === 1 ? " hour ago" : " hours ago");
    }
    const d = Math.floor(secs / 86400);
    if (d < 31)
        return d === 1 ? "yesterday" : d + " days ago";
    return new Date(ms).toISOString().slice(0, 10);
}

// ---- Checking what was typed, with the same rules as page.toml's argument patterns ----

var CONTROL = /[\u0000-\u001f\u007f-\u009f]/;

function triggerProblem(text, macros) {
    const t = String(text).trim();
    if (t === "")
        return "A macro needs a trigger.";
    if (t.length > 256)
        return "A trigger is at most 256 characters.";
    if (CONTROL.test(t))
        return "A trigger can't contain control characters.";
    if (t.charAt(0) === "-")
        return "A trigger can't start with a dash here; add it in emobie.";
    if ((macros || []).some(m => m.trigger === t))
        return "Another macro already uses this trigger.";
    return "";
}

function expansionProblem(text) {
    const s = String(text);
    if (s === "")
        return "A macro needs an expansion.";
    if (s.length > 1000)
        return "From here an expansion is at most 1000 characters; use emobie for longer ones.";
    if (/[\u0000-\u001f\u007f]/.test(s))
        return "Longer or multi-line text is added in emobie.";
    return "";
}

function appProblem(text, apps) {
    const t = String(text).trim();
    if (t === "")
        return "Type the name of an app.";
    if (t.length > 128)
        return "An app name is at most 128 characters.";
    if (CONTROL.test(t) || t.charAt(0) === "-")
        return "That isn't an app name.";
    if ((apps || []).some(a => String(a).toLowerCase() === t.toLowerCase()))
        return t + " is already in the list.";
    return "";
}

// An id the page's commands accept (page.toml); an odd one can only be changed in emobie.
function idOk(id) {
    return typeof id === "string" && /^[A-Za-z0-9._:][A-Za-z0-9._:-]{0,79}$/.test(id);
}

function counted(n, one, many) {
    return n + " " + (n === 1 ? one : many);
}
