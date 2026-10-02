/** Réglages système lus dans Rust ; null signifie état autostart indisponible. */
export interface DesktopSettings {
  closeToTray: boolean;
  autostartEnabled: boolean | null;
  trayAvailable: boolean;
  storageError: boolean;
}
export type DesktopSettingKey = 'closeToTray' | 'autostartEnabled';
export type DesktopSettingsError = 'unavailable' | 'read_failed' | 'write_failed' | 'autostart_failed' | 'verification_failed';
