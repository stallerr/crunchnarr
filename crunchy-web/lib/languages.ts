export const LANGUAGE_NAMES: Record<string, string> = {
  'ja-JP': 'Japanese',
  'en-US': 'English (US)',
  'en-GB': 'English (UK)',
  'es-LA': 'Spanish (Latin America)',
  'es-419': 'Spanish (Latin America)',
  'es-ES': 'Spanish (Spain)',
  'ru-RU': 'Russian',
  'pt-BR': 'Portuguese (Brazil)',
  'pt-PT': 'Portuguese (Portugal)',
  'fr-FR': 'French',
  'de-DE': 'German',
  'it-IT': 'Italian',
  'ar-SA': 'Arabic',
  'hi-IN': 'Hindi',
  'zh-CN': 'Chinese (Simplified)',
  'zh-TW': 'Chinese (Traditional)',
  'ko-KR': 'Korean',
  'id-ID': 'Indonesian',
  'ms-MY': 'Malay',
  'th-TH': 'Thai',
  'vi-VN': 'Vietnamese',
  'pl-PL': 'Polish',
  'tr-TR': 'Turkish',
};

export function getLanguageName(code: string): string {
  return LANGUAGE_NAMES[code] ?? code;
}
