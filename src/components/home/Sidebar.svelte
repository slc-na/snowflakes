<script>
  import {
    House,
    Folders,
    ShieldCheck,
    ArrowLeftRight,
    Settings,
    Clock,
  } from "@lucide/svelte";
  import SidebarElement from "./SidebarElement.svelte";

  const menus = [
    { icon: House, text: "HOME", href: "/", isActive: true },
    { icon: Clock, text: "RECENTS", href: "/recents", isActive: false },
    { icon: Folders, text: "FILES", href: "/files", isActive: false },
    // { icon: ShieldCheck, text: "KNOWN HOSTS", href: "/hosts", isActive: false },
    // {
    //   icon: ArrowLeftRight,
    //   text: "PORT FORWARDING",
    //   href: "/ports",
    //   isActive: false,
    // },
    { icon: Settings, text: "SETTINGS", href: "/settings", isActive: false },
  ];
</script>

<div class="sidebar-container">
  <div class="brand-section">
    <h1 class="brand-title">SNOWFLAKES</h1>
    <p class="brand-subtitle">SSH MANAGER</p>
  </div>

  <nav class="nav-menu">
    {#each menus as menu}
      <SidebarElement
        text={menu.text}
        icon={menu.icon}
        href={menu.href}
        isActive={menu.isActive}
      />
    {/each}
  </nav>
</div>

<style>
  .sidebar-container {
    height: calc(100vh - 32px);
    width: var(--sf-sidebar-width);
    background-color: var(--sf-bg-sidebar);
    border-right: 1px solid var(--sf-border);
    display: flex;
    flex-direction: column;
    padding-top: var(--sf-space-xl);
  }

  .brand-section {
    padding: 0 var(--sf-space-lg);
    margin-bottom: var(--sf-space-xl);
  }

  .brand-title {
    font-family: var(--sf-font-title);
    font-size: 1.25rem;
    font-weight: 800;
    letter-spacing: 0.1em;
    color: var(--sf-accent);
    margin: 0;
  }

  .brand-subtitle {
    font-family: var(--sf-font-ui);
    font-size: 0.7rem;
    font-weight: 700;
    color: var(--sf-text-secondary);
    margin: 4px 0 0 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .nav-menu {
    display: flex;
    flex-direction: column;
  }
</style>
