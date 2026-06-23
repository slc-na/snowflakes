<script lang="ts">
  import { goto } from "$app/navigation";

  let { text, icon, href, isActive } = $props();
  let isHovered = $state(false);

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      goto(href);
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="parent {isActive || isHovered ? 'active' : ''}"
  onclick={() => goto(href)}
  onkeydown={handleKeydown}
  role="button"
  tabindex="0"
  onmouseenter={() => (isHovered = true)}
  onmouseleave={() => (isHovered = false)}
>
  <!-- svelte-ignore svelte_component_deprecated -->
  <div class="icon-wrap">
    <svelte:component this={icon} size={24} />
  </div>
  <span class="menu">{text}</span>
</div>

<style>
  .parent {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    border-radius: var(--sf-radius-sm);
    padding: 10px;
  }
  .menu {
    font-family: var(--sf-font-ui);
    color: var(--sf-text-primary);
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .icon-wrap {
    color: var(--sf-text-primary);
  }
  .active {
    background-color: var(--sf-bg-hover);
  }
</style>
