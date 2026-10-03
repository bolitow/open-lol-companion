export type DiagnosticLocale='fr'|'en';
export type DiagnosticExportResult='exported'|'cancelled';
export type DiagnosticExportError='busy'|'unavailable'|'read_failed'|'write_failed'|'too_large'|'unsupported';
export interface DiagnosticExportRequest {includeLeagueSummary:boolean;locale:DiagnosticLocale}
