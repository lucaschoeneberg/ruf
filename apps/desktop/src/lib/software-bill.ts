/** The target and feature selection used by the desktop build. */
export interface SoftwareBuild {
  target: string;
  native: boolean;
  profile: "debug" | "release";
  kind: "desktop" | "preview";
}
export interface SoftwareProperty {
  name: string;
  value: string;
}
export interface SoftwareComponent {
  type: "library" | "application" | "file";
  "bom-ref": string;
  name: string;
  group?: string;
  version: string;
  purl?: string;
  scope?: "required" | "excluded";
  publisher?: string;
  properties?: SoftwareProperty[];
  hashes?: { alg: "SHA-256"; content: string }[];
  licenses?: { expression: string }[];
}
/** Target-specific resolved dependency graph in CycloneDX 1.6 format. */
export interface SoftwareBill {
  bomFormat: "CycloneDX";
  specVersion: "1.6";
  version: 1;
  serialNumber: string;
  metadata: {
    timestamp: string;
    tools: {
      components: { type: "application"; name: string; version: string }[];
    };
    component: SoftwareComponent;
    properties: SoftwareProperty[];
  };
  components: SoftwareComponent[];
  dependencies: { ref: string; dependsOn: string[] }[];
}
