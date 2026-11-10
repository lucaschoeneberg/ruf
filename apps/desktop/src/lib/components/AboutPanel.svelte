<script lang="ts">
  import { t, text, message, type MessageValue } from "../i18n";
  import Changelog from "./Changelog.svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import initialBill from "virtual:software-bill";
  import type { SoftwareBill } from "../software-bill";
  import { openAboutLink, getSoftwareBill } from "../api";
  import type { AboutLink, AppInfo } from "../generated/AppInfo";
  import license from "../../../../../LICENSE?raw";
  import notices from "../../../../../THIRD_PARTY_NOTICES.md?raw";
  import fontLicense from "/fonts/IBM-Plex-Sans-LICENSE.txt?raw";

  let { info }: { info: AppInfo | null } = $props();
  let linkError = $state<MessageValue | null>(null);
  let bill = $state<SoftwareBill>(initialBill);
  let billLoaded = $state(false);
  let billError = $state(false);
  const native = $derived(
    bill.metadata.properties.some(
      (item) => item.name === "ruf:build:native" && item.value === "true",
    ),
  );
  const openssl = $derived(
    bill.components.find((item) => item.name === "openssl"),
  );
  onMount(() => {
    const controller = new AbortController();
    const localFile = async () => {
      const response = await fetch("/software-bill.cdx.json", {
        signal: controller.signal,
        cache: "no-cache",
      });
      if (!response.ok) throw new Error("SBOM unavailable");
      return response.json() as Promise<SoftwareBill>;
    };
    const load = isTauri()
      ? getSoftwareBill()
          .then((json) => JSON.parse(json) as SoftwareBill)
          .catch((error: unknown) => {
            if (import.meta.env.DEV) return localFile();
            throw error;
          })
      : localFile();
    void load
      .then((data) => {
        if (
          data.bomFormat !== "CycloneDX" ||
          !Array.isArray(data.components) ||
          !Array.isArray(data.dependencies)
        )
          throw new Error("Invalid SBOM");
        if (!controller.signal.aborted) {
          bill = data;
          billLoaded = true;
        }
      })
      .catch(() => {
        if (!controller.signal.aborted) billError = true;
      });
    return () => controller.abort();
  });
  const links: { id: AboutLink; label: MessageValue; href: string }[] = [
    {
      id: "company",
      label: message("settings.website"),
      href: "https://www.mardu.de/",
    },
    {
      id: "imprint",
      label: message("settings.imprint"),
      href: "https://www.mardu.de/publisher",
    },
    {
      id: "privacy",
      label: message("settings.privacy"),
      href: "https://www.mardu.de/privacy",
    },
  ];
  const primary = [
    {
      name: "Svelte",
      package: "pkg:npm/svelte@",
      role: message("settings.ui_role"),
      license: "MIT",
    },
    {
      name: "Tauri",
      package: "pkg:cargo/tauri@",
      role: message("settings.desktop_role"),
      license: "MIT / Apache-2.0",
    },
    {
      name: "baresip",
      package: "pkg:github/baresip/baresip@",
      role: message("settings.sip_audio_role"),
      license: "BSD-3-Clause",
    },
    {
      name: "libre",
      package: "pkg:github/baresip/re@",
      role: message("settings.sip_library_role"),
      license: "BSD-3-Clause",
    },
    {
      name: "rusqlite",
      package: "pkg:cargo/rusqlite@",
      role: message("settings.history_role"),
      license: "MIT · SQLite: Public Domain",
    },
    {
      name: "keyring",
      package: "pkg:cargo/keyring@",
      role: message("settings.keychain_role"),
      license: "MIT / Apache-2.0",
    },
  ];
  const licenseFiles = Object.entries(
    import.meta.glob<string>("../../../../../native/licenses/*", {
      query: "?raw",
      import: "default",
      eager: true,
    }),
  ).map(([path, text]) => ({ name: path.split("/").at(-1) ?? path, text }));

  async function openLink(event: MouseEvent, link: AboutLink) {
    if (!isTauri()) return;
    event.preventDefault();
    linkError = null;
    try {
      await openAboutLink(link);
    } catch {
      linkError = message("settings.link_failed");
    }
  }
</script>

<div class="settings-title">
  <div>
    <div class="about-heading">
      <img src="/ruf.svg" alt="" width="28" height="28" />
      <h1 id="about-heading" tabindex="-1">{$t("settings.about_title")}</h1>
    </div>
    <p>{$t("settings.about_description")}</p>
  </div>
</div>
<h2 class="settings-group">{$t("settings.this_version")}</h2>
<div class="setting-row">
  <div class="setting-description">
    <h3>{$t("settings.version")}</h3>
    <p>{$t("settings.open_source_license")}</p>
  </div>
  <span>{info?.version ?? bill.metadata.component.version}</span>
</div>
<Changelog />
<div class="setting-row">
  <div class="setting-description">
    <h3>{$t("settings.telephony")}</h3>
    <p>
      {$t("settings.telephony_description")}
    </p>
  </div>
</div>
<details class="settings-lab">
  <summary id="software-licenses">{$t("settings.software_licenses")}</summary>
  <p>{$t("settings.software_bill")}</p>
  <ul class="about-components" aria-label={$t("settings.core_components")}>
    {#each primary as item}
      {@const component = bill.components.find((component) =>
        component.purl?.startsWith(item.package),
      )}
      {#if component}
        <li>
          <span>{item.name} <small>{component?.version ?? "—"}</small></span
          ><span>{item.license}</span>
        </li>
      {/if}
    {/each}
    {#if openssl}<li>
        <span>OpenSSL <small>{openssl.version}</small></span><span
          >Apache-2.0</span
        >
      </li>{/if}
    <li><span>IBM Plex Sans</span><span>SIL OFL-1.1</span></li>
  </ul>
  <p>
    {native
      ? $t("settings.native_build_details")
      : $t("settings.no_native_build")}
  </p>
  <details class="about-license-text">
    <summary>{$t("settings.license_texts")}</summary>
    <p>
      {$t("settings.license_offline")}
    </p>
    <details class="about-license-text">
      <summary>{$t("settings.ruf_license_heading")}</summary>
      <textarea
        readonly
        rows="12"
        aria-label={$t("settings.ruf_license_label")}
        value={license}></textarea>
    </details>
    <details class="about-license-text">
      <summary>{$t("settings.third_party_heading")}</summary>
      <textarea
        readonly
        rows="12"
        aria-label={$t("settings.third_party_label")}
        value={notices}></textarea>
    </details>
    <details class="about-license-text">
      <summary>{$t("settings.font_license_heading")}</summary>
      <textarea
        readonly
        rows="12"
        aria-label={$t("settings.font_license_label")}
        value={fontLicense}></textarea>
    </details>
    {#each licenseFiles as file}
      <details class="about-license-text">
        <summary>{file.name}</summary>
        <textarea readonly rows="12" aria-label={file.name} value={file.text}
        ></textarea>
      </details>
    {/each}
  </details>

  <details class="about-license-text">
    <summary id="software-inventory">{$t("settings.project_inventory")}</summary
    >
    <p>
      {$t("settings.inventory_description")}
    </p>
    {#if billLoaded}<textarea
        readonly
        rows="8"
        aria-label={$t("settings.sbom_label")}
        value={JSON.stringify(bill, null, 2)}></textarea>
    {:else}<p role="status">
        {billError ? $t("settings.sbom_failed") : $t("settings.sbom_loading")}
      </p>{/if}
  </details>
</details>
<footer class="about-footer">
  <p>{$t("settings.author")}</p>
  <nav class="about-links" aria-label="Mardu GmbH">
    {#each links as link}
      <a
        href={link.href}
        target="_blank"
        rel="noopener noreferrer"
        onclick={(event) => openLink(event, link.id)}>{$text(link.label)}</a
      >
    {/each}
  </nav>
</footer>
{#if linkError}<p class="about-error" role="alert">{$text(linkError)}</p>{/if}

<style>
  .about-heading {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 5px;
  }
  .about-heading h1 {
    margin: 0;
  }
  .about-components {
    list-style: none;
    padding: 0;
    margin: 10px 0;
  }
  .about-components li {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 5px 0;
  }
  .about-components li > span {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .about-components li > span:last-child {
    text-align: right;
  }
  .about-components small {
    margin-left: 4px;
    color: var(--muted);
    font-size: 11px;
  }
  .about-license-text {
    margin-top: 12px;
  }
  .about-license-text textarea {
    display: block;
    width: 100%;
    margin-top: 8px;
    resize: vertical;
    background: var(--surface-panel);
    font-family: monospace;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 240px;
    overflow-y: auto;
    font-size: 11px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
  }
  .about-footer {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 6px 16px;
    padding-top: 16px;
    font-size: 11px;
    color: var(--muted);
  }
  .about-footer p {
    margin: 0;
  }
  .about-links {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 12px;
  }
  .about-links a {
    color: var(--muted);
    text-underline-offset: 3px;
  }
  .about-links a:hover {
    color: var(--text);
  }
  .about-error {
    color: var(--danger);
    font-size: 12px;
  }
  a:focus-visible,
  summary:focus-visible,
  textarea:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }
</style>
