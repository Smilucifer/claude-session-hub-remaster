<script lang="ts">
  let {
    value = $bindable(""),
    placeholder = "",
    type = "text",
    disabled = false,
    label = "",
    id = "",
    class: className = "",
    oninput,
    onblur,
    onpaste,
  }: {
    value?: string;
    placeholder?: string;
    type?: string;
    disabled?: boolean;
    label?: string;
    id?: string;
    class?: string;
    oninput?: (e: Event) => void;
    onblur?: (e: FocusEvent) => void;
    onpaste?: (e: ClipboardEvent) => void;
  } = $props();

  const inputId = id || (label ? `input-${label.toLowerCase().replace(/\s+/g, '-')}` : '');
</script>

{#if label}
  <label for={inputId} class="text-sm font-medium leading-none mb-1.5 block">{label}</label>
{/if}
<input
  id={inputId}
  {type}
  {placeholder}
  {disabled}
  bind:value
  {oninput}
  {onblur}
  {onpaste}
  class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 {className}"
/>
