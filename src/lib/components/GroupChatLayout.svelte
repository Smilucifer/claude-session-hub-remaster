<script lang="ts">
  import type { GroupChatRunIndexEntry } from "$lib/api";

  let { groupChat }: { groupChat: GroupChatRunIndexEntry | null } = $props();
</script>

<div class="flex h-full items-center justify-center">
  <div class="text-center text-muted-foreground">
    <p class="text-lg font-medium">{groupChat?.room_name ?? "Group Chat"}</p>
    <p class="text-sm">Group chat view — coming soon</p>
  </div>
</div>
