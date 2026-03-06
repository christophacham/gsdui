/**
 * Settings store with localStorage persistence using Svelte 5 runes.
 *
 * Stores user display preferences for the pipeline dashboard.
 * All settings are persisted to localStorage under 'gsdui-settings' key.
 */

const STORAGE_KEY = 'gsdui-settings';

interface NotificationSettings {
	planCompletion: boolean;
	errors: boolean;
	phaseTransitions: boolean;
	agentSwitches: boolean;
}

interface VisibleStats {
	steps: boolean;
	commits: boolean;
	duration: boolean;
	wave: boolean;
}

interface PersistedSettings {
	outputLineCount: number;
	defaultCardState: 'collapsed' | 'expanded';
	autoExpandActive: boolean;
	visibleStats: VisibleStats;
	autoScroll: boolean;
	timelineDensity: 'rich' | 'medium' | 'minimal';
	fontSize: number;
	notifications: NotificationSettings;
}

function loadSettings(): Partial<PersistedSettings> {
	if (typeof localStorage === 'undefined') return {};

	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return {};
		return JSON.parse(raw) as Partial<PersistedSettings>;
	} catch {
		return {};
	}
}

class SettingsStore {
	/** Number of output lines visible in expanded plan cards */
	outputLineCount = $state(10);

	/** Default card state for plan cards */
	defaultCardState = $state<'collapsed' | 'expanded'>('collapsed');

	/** Auto-expand the currently active plan card */
	autoExpandActive = $state(true);

	/** Which stats to show on plan cards */
	visibleStats = $state<VisibleStats>({
		steps: true,
		commits: true,
		duration: true,
		wave: true
	});

	/** Auto-scroll output stream in expanded cards */
	autoScroll = $state(true);

	/** Timeline chip density */
	timelineDensity = $state<'rich' | 'medium' | 'minimal'>('rich');

	/** Base font size in pixels */
	fontSize = $state(14);

	/** Notification preferences */
	notifications = $state<NotificationSettings>({
		planCompletion: true,
		errors: true,
		phaseTransitions: true,
		agentSwitches: true
	});

	constructor() {
		const saved = loadSettings();

		if (saved.outputLineCount !== undefined) this.outputLineCount = saved.outputLineCount;
		if (saved.defaultCardState !== undefined) this.defaultCardState = saved.defaultCardState;
		if (saved.autoExpandActive !== undefined) this.autoExpandActive = saved.autoExpandActive;
		if (saved.visibleStats !== undefined) this.visibleStats = saved.visibleStats;
		if (saved.autoScroll !== undefined) this.autoScroll = saved.autoScroll;
		if (saved.timelineDensity !== undefined) this.timelineDensity = saved.timelineDensity;
		if (saved.fontSize !== undefined) this.fontSize = saved.fontSize;
		if (saved.notifications !== undefined) this.notifications = saved.notifications;

		// Persist on any change
		$effect(() => {
			this.persist();
		});
	}

	private persist(): void {
		if (typeof localStorage === 'undefined') return;

		const data: PersistedSettings = {
			outputLineCount: this.outputLineCount,
			defaultCardState: this.defaultCardState,
			autoExpandActive: this.autoExpandActive,
			visibleStats: this.visibleStats,
			autoScroll: this.autoScroll,
			timelineDensity: this.timelineDensity,
			fontSize: this.fontSize,
			notifications: this.notifications
		};

		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
		} catch (err) {
			console.error('[SettingsStore] Failed to persist settings:', err);
		}
	}
}

export const settingsStore = new SettingsStore();
