declare module "virtual:software-bill" {
  const bill: import("./lib/software-bill").SoftwareBill;
  export default bill;
}

interface ImportMetaEnv {
  readonly VITE_RUF_CHANGELOG_URL?: string;
  readonly VITE_RUF_CHANGELOG_URL_DE?: string;
}
