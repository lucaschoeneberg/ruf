; Match the installed-release COM activator in baresip-adapter/notifications/windows.rs.
; Notifications are per-user; never touch another application's registrations.
!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegKey HKCU "Software\Classes\CLSID\{A14373DB-08E7-45EA-9E24-26CB70BFA7B1}"
  DeleteRegKey HKCU "Software\Classes\AppUserModelId\${BUNDLEID}"
!macroend
