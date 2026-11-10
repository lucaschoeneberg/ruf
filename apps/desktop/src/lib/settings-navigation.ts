import { formatMessage, getLocale, message, type MessageValue } from "./i18n";
import type { AppLocale } from "./generated/AppInfo";
import type { SettingsSection } from "./api";
export const settingsSections = [
  { id: "general", label: message("navigation.general"), icon: "settings" },
  {
    id: "accounts",
    label: message("navigation.sip_accounts"),
    icon: "contacts",
  },
  { id: "audio", label: message("navigation.sound"), icon: "audio" },
  {
    id: "notifications",
    label: message("navigation.notifications"),
    icon: "info",
  },
  {
    id: "contacts",
    label: message("navigation.contacts_history"),
    icon: "contacts",
  },
  { id: "privacy", label: message("navigation.privacy"), icon: "info" },
  {
    id: "diagnostics",
    label: message("navigation.diagnostics_help"),
    icon: "settings",
  },
  { id: "about", label: message("navigation.about_ruf"), icon: "info" },
] as const;
export type SettingResult = {
  section: SettingsSection;
  target: string;
  label: string;
  terms: string;
};
type LocalizedSettingResult = Omit<SettingResult, "label" | "terms"> & {
  label: MessageValue;
  terms: MessageValue;
};
const fields: LocalizedSettingResult[] = [
  {
    section: "general",
    target: "language-preference",
    label: message("navigation.language"),
    terms: message("navigation.language_terms"),
  },
  {
    section: "diagnostics",
    target: "keyboard-shortcuts",
    label: message("navigation.keyboard_shortcuts"),
    terms: message("navigation.keyboard_shortcuts_commands_actions_search"),
  },
  {
    section: "general",
    target: "autostart-enabled",
    label: message("navigation.start_at_login"),
    terms: message("navigation.autostart_login_start_close"),
  },
  {
    section: "general",
    target: "start-hidden",
    label: message("navigation.start_in_the_background_at_login"),
    terms: message("navigation.autostart_login_background_window"),
  },
  {
    section: "accounts",
    target: "name",
    label: message("navigation.edit_sip_account"),
    terms: message("navigation.accounts_account_name_connection_default"),
  },
  {
    section: "accounts",
    target: "registrar",
    label: message("navigation.sip_server"),
    terms: message("navigation.server_registrar_host_domain_register_connect"),
  },
  {
    section: "accounts",
    target: "username",
    label: message("navigation.username"),
    terms: message("navigation.login_extension_username"),
  },
  {
    section: "accounts",
    target: "password",
    label: message("navigation.password"),
    terms: message("navigation.credentials_keychain_password"),
  },
  {
    section: "accounts",
    target: "transport",
    label: message("navigation.sip_transport"),
    terms: message("navigation.udp_tcp_tls_encryption_certificate_connection"),
  },
  {
    section: "accounts",
    target: "media-security",
    label: message("navigation.audio_security"),
    terms: message("navigation.srtp_sdes_security_encryption_tls"),
  },
  {
    section: "accounts",
    target: "auth-username",
    label: message("navigation.separate_authentication_username"),
    terms: message("navigation.auth_username_digest_authentication"),
  },
  {
    section: "accounts",
    target: "outbound-proxy",
    label: message("navigation.outbound_proxy"),
    terms: message("navigation.proxy_server_routing_connection"),
  },
  {
    section: "audio",
    target: "inputDevice",
    label: message("navigation.microphone"),
    terms: message("navigation.audio_sound_input"),
  },
  {
    section: "audio",
    target: "outputDevice",
    label: message("navigation.speaker"),
    terms: message("navigation.audio_sound_output_call"),
  },
  {
    section: "audio",
    target: "ringDevice",
    label: message("navigation.ringtone_output"),
    terms: message("navigation.ring_device_output"),
  },
  {
    section: "audio",
    target: "ringtone-enabled",
    label: message("navigation.ring_for_incoming_calls"),
    terms: message("navigation.ringtone_ring_on_off"),
  },
  {
    section: "audio",
    target: "ringtone-style",
    label: message("navigation.ringtone"),
    terms: message("navigation.classic_soft_short_ringtone"),
  },
  {
    section: "audio",
    target: "ringtone-volume",
    label: message("navigation.ringtone_volume"),
    terms: message("navigation.ringtone_volume_quiet_loud"),
  },
  {
    section: "audio",
    target: "audio-tests",
    label: message("navigation.audio_tests"),
    terms: message("navigation.microphone_test_level_speaker_listen"),
  },
  {
    section: "notifications",
    target: "notification-permission",
    label: message("navigation.notification_permission"),
    terms: message("navigation.notifications_allow_permission_system"),
  },
  {
    section: "notifications",
    target: "notifications-enabled",
    label: message("navigation.show_incoming_calls"),
    terms: message("navigation.notifications_enable"),
  },
  {
    section: "notifications",
    target: "notification-caller",
    label: message("navigation.show_caller_details"),
    terms: message("navigation.name_number_privacy_notifications"),
  },
  {
    section: "notifications",
    target: "test-notification",
    label: message("navigation.send_test_notification"),
    terms: message("navigation.notification_test"),
  },
  {
    section: "contacts",
    target: "contacts-display",
    label: message("navigation.contact_display"),
    terms: message("navigation.contacts_address_book_display"),
  },
  {
    section: "contacts",
    target: "favorites-first",
    label: message("navigation.favorites_first"),
    terms: message("navigation.favorites_sort_order"),
  },
  {
    section: "contacts",
    target: "match-german-numbers",
    label: message("navigation.match_german_dialing_prefixes"),
    terms: message(
      "navigation.country_code_germany_49_0049_number_contact_match",
    ),
  },
  {
    section: "contacts",
    target: "show-organization",
    label: message("navigation.show_organization"),
    terms: message("navigation.organization_contact_company"),
  },
  {
    section: "contacts",
    target: "history-recording",
    label: message("navigation.save_call_history"),
    terms: message("navigation.history_calls_record_metadata"),
  },
  {
    section: "contacts",
    target: "history-retention",
    label: message("navigation.retention"),
    terms: message("navigation.history_unlimited_30_90_365_days_retention"),
  },
  {
    section: "contacts",
    target: "clear-history",
    label: message("navigation.clear_history"),
    terms: message("navigation.history_clear_remove_delete"),
  },
  {
    section: "privacy",
    target: "privacy-permissions",
    label: message("navigation.permissions"),
    terms: message(
      "navigation.microphone_permission_privacy_access_keychain_storage",
    ),
  },
  {
    section: "privacy",
    target: "contacts-permission",
    label: message("navigation.allow_contacts"),
    terms: message("navigation.macos_address_book_permission_privacy_access"),
  },
  {
    section: "diagnostics",
    target: "diagnostics-status",
    label: message("navigation.connection_and_help"),
    terms: message(
      "navigation.diagnostics_error_connection_status_export_shortcuts",
    ),
  },
  {
    section: "diagnostics",
    target: "developer-audio",
    label: message("navigation.developer_audio"),
    terms: message("navigation.audio_source_test_tone_development_recording"),
  },
  {
    section: "about",
    target: "about-heading",
    label: message("navigation.version_and_software"),
    terms: message(
      "navigation.about_ruf_changelog_changes_author_mardu_company_imprint",
    ),
  },
  {
    section: "about",
    target: "software-licenses",
    label: message("navigation.software_licenses"),
    terms: message("navigation.open_source_licenses_origin"),
  },
  {
    section: "about",
    target: "software-inventory",
    label: message("navigation.project_inventory_sbom"),
    terms: message("navigation.software_bill_of_materials_sbom_components"),
  },
];

export function searchSettings(
  query: string,
  language: AppLocale = getLocale(),
): SettingResult[] {
  const words = query
    .trim()
    .toLocaleLowerCase(language)
    .split(/\s+/)
    .filter(Boolean);
  if (!words.length) return [];
  return fields
    .map((field) => ({
      ...field,
      label: formatMessage(language, field.label),
      terms: formatMessage(language, field.terms),
    }))
    .filter((field) => {
      const text =
        `${formatMessage(language, field.label)} ${formatMessage(language, field.terms)}`.toLocaleLowerCase(
          language,
        );
      return words.every((word) => text.includes(word));
    });
}
