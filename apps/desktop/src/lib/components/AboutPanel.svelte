<script lang="ts">
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
  let linkError = $state<string | null>(null);
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
  const links: { id: AboutLink; label: string; href: string }[] = [
    { id: "company", label: "Website", href: "https://www.mardu.de/" },
    {
      id: "imprint",
      label: "Impressum",
      href: "https://www.mardu.de/publisher",
    },
    {
      id: "privacy",
      label: "Datenschutz",
      href: "https://www.mardu.de/privacy",
    },
  ];
  const primary = [
    {
      name: "Svelte",
      package: "pkg:npm/svelte@",
      role: "Oberfläche",
      license: "MIT",
    },
    {
      name: "Tauri",
      package: "pkg:cargo/tauri@",
      role: "Desktop-App",
      license: "MIT / Apache-2.0",
    },
    {
      name: "baresip",
      package: "pkg:github/baresip/baresip@",
      role: "SIP & Audio",
      license: "BSD-3-Clause",
    },
    {
      name: "libre",
      package: "pkg:github/baresip/re@",
      role: "SIP-Bibliothek",
      license: "BSD-3-Clause",
    },
    {
      name: "rusqlite",
      package: "pkg:cargo/rusqlite@",
      role: "Lokaler Anrufverlauf · SQLite",
      license: "MIT · SQLite: Public Domain",
    },
    {
      name: "keyring",
      package: "pkg:cargo/keyring@",
      role: "Passwörter im Systemschlüsselbund",
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
      linkError =
        "Die Seite konnte nicht geöffnet werden. Öffne www.mardu.de in deinem Browser.";
    }
  }
</script>

<div class="settings-title">
  <div>
    <div class="about-heading">
      <img src="/ruf.svg" alt="" width="28" height="28" />
      <h1 id="about-heading" tabindex="-1">Über Ruf</h1>
    </div>
    <p>Ein kompaktes Telefon für deinen Desktop.</p>
  </div>
</div>
<h2 class="settings-group">Diese Version</h2>
<div class="setting-row">
  <div class="setting-description">
    <h3>Version</h3>
    <p>Open Source · MIT-Lizenz</p>
  </div>
  <span>{info?.version ?? bill.metadata.component.version}</span>
</div>
<Changelog />
<div class="setting-row">
  <div class="setting-description">
    <h3>Telefonie</h3>
    <p>
      Derzeit SIP/UDP ohne Verschlüsselung und G.711. Nutze diese
      Entwicklungsversion nur im isolierten Testnetz. Ein Gespräch gleichzeitig.
    </p>
  </div>
</div>
<details class="settings-lab">
  <summary id="software-licenses">Software & Lizenzen</summary>
  <p>Software Bill of Materials (SBOM)</p>
  <ul class="about-components" aria-label="Zentrale Softwarekomponenten">
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
      ? "Dieser Build enthält native Telefonie mit baresip, libre und OpenSSL."
      : "Dieser Build enthält keine native Telefonie."}
  </p>
  <details class="about-license-text">
    <summary>Lizenztexte & Herkunftshinweise</summary>
    <p>
      Die folgenden Texte werden mit Ruf ausgeliefert und sind auch offline
      verfügbar.
    </p>
    <details class="about-license-text">
      <summary>Ruf · MIT-Lizenz</summary>
      <textarea readonly rows="12" aria-label="Ruf MIT-Lizenz" value={license}
      ></textarea>
    </details>
    <details class="about-license-text">
      <summary>Third-party notices · Herkunft & Verwendung</summary>
      <textarea
        readonly
        rows="12"
        aria-label="Third-party notices"
        value={notices}></textarea>
    </details>
    <details class="about-license-text">
      <summary>IBM Plex Sans · SIL Open Font License</summary>
      <textarea
        readonly
        rows="12"
        aria-label="IBM Plex Sans Lizenz"
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
    <summary id="software-inventory">Projektinventar (SBOM)</summary>
    <p>
      Komponenten und Abhängigkeitsbeziehungen für diesen Build. Reine
      Build-Abhängigkeiten sind mit „excluded“ gekennzeichnet.
    </p>
    {#if billLoaded}<textarea
        readonly
        rows="8"
        aria-label="SBOM als CycloneDX-JSON"
        value={JSON.stringify(bill, null, 2)}></textarea>
    {:else}<p role="status">
        {billError
          ? "Die SBOM konnte nicht geladen werden."
          : "SBOM wird geladen …"}
      </p>{/if}
  </details>
</details>
<footer class="about-footer">
  <p>Autor: Mardu GmbH</p>
  <nav class="about-links" aria-label="Mardu GmbH">
    {#each links as link}
      <a
        href={link.href}
        target="_blank"
        rel="noopener noreferrer"
        onclick={(event) => openLink(event, link.id)}>{link.label}</a
      >
    {/each}
  </nav>
</footer>
{#if linkError}<p class="about-error" role="alert">{linkError}</p>{/if}

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
