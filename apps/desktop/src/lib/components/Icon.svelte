<script lang="ts">
  const paths = {
    phone: [
      "M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72c.13.96.36 1.9.69 2.79a2 2 0 0 1-.45 2.11L8.08 9.89a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45c.89.33 1.83.56 2.79.69A2 2 0 0 1 22 16.92Z",
    ],
    endCall: [
      "M4 15h3l1-4a17 17 0 0 1 8 0l1 4h3a2 2 0 0 0 2-2v-2c-5-6-15-6-20 0v2a2 2 0 0 0 2 2Z",
    ],
    contacts: [
      "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2",
      "M16 3a4 4 0 0 1 0 8",
      "M22 21v-2a4 4 0 0 0-3-3.87",
      "M13 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z",
    ],
    history: ["M3 11a9 9 0 1 1 2.6 7.4", "M3 4v7h7", "M12 7v5l3 2"],
    keypad: [
      "M5 4h1v1H5zM11.5 4h1v1h-1zM18 4h1v1h-1zM5 10.5h1v1H5zM11.5 10.5h1v1h-1zM18 10.5h1v1h-1zM5 17h1v1H5zM11.5 17h1v1h-1zM18 17h1v1h-1z",
    ],
    settings: [
      "M9.5 2h5l.5 3 2 1 2.6-1 2.5 4.4-2.3 2V13l2.3 2-2.5 4.4-2.6-1-2 1-.5 3h-5l-.5-3-2-1-2.6 1L1 15l2.3-2v-1.6L1 9.4 3.5 5l2.6 1 2-1 .5-3Z",
      "M15.5 12a3.5 3.5 0 1 1-7 0 3.5 3.5 0 0 1 7 0Z",
    ],
    backspace: [
      "M9 5h11a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H9L2 12l7-7Z",
      "m12 9 6 6m0-6-6 6",
    ],
    back: ["m15 18-6-6 6-6"],
    close: ["m6 6 12 12M18 6 6 18"],
    plus: ["M12 5v14M5 12h14"],
    search: ["M21 21l-5-5", "M18 10.5a7.5 7.5 0 1 1-15 0 7.5 7.5 0 0 1 15 0Z"],
    refresh: [
      "M20 7v5h-5",
      "M4 17v-5h5",
      "M6.1 6.1A8 8 0 0 1 20 12M4 12a8 8 0 0 0 13.9 5.9",
    ],
    star: [
      "m12 3 2.8 5.7 6.3.9-4.6 4.4 1.1 6.3-5.6-3-5.6 3 1.1-6.3-4.6-4.4 6.3-.9L12 3Z",
    ],
    chevronRight: ["m9 6 6 6-6 6"],
    chevronDown: ["m6 9 6 6 6-6"],
    audio: [
      "M11 5 6 9H3v6h3l5 4V5Z",
      "M15 8a6 6 0 0 1 0 8M18 5a10 10 0 0 1 0 14",
    ],
    info: ["M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z", "M12 11v6M12 7h.01"],
  } satisfies Record<string, string[]>;
  let { name, size = 20 }: { name: keyof typeof paths; size?: number } =
    $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#each paths[name] as path}<path d={path} />{/each}
</svg>
