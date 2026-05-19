<script>
	import { page } from "$app/stores";
	import {
		House,
		Folders,
		ShieldCheck,
		ArrowLeftRight,
		Settings,
		LayoutGrid,
	} from "@lucide/svelte";
	import SidebarElement from "../components/home/SidebarElement.svelte";
	import "../layout.css";
	import TitleBar from "../components/TitleBar.svelte";
	import SessionTabBar from "../components/terminal/SessionTabBar.svelte";
	import { Toaster } from 'svelte-sonner';

	const menus = [
		{ icon: House, text: "HOME", href: "/" },
		{ icon: Folders, text: "FILES", href: "/files" },
		{ icon: ShieldCheck, text: "KNOWN HOSTS", href: "/hosts" },
		{ icon: ArrowLeftRight, text: "PORT FORWARDING", href: "/ports" },
		{ icon: LayoutGrid, text: "MULTI WINDOW", href: "/window" },
		{ icon: Settings, text: "SETTINGS", href: "/settings" },
	];

	let showTabBar = $derived(menus.some(
		(menu) =>
			menu.href === $page.url.pathname ||
			$page.url.pathname.includes("/session"),
	));
	
	let isLoginPage = $derived($page.url.pathname === '/login');
</script>

<TitleBar />

<Toaster
	toastOptions={{
		style: 'background: var(--sf-bg-surface); color: var(--sf-text-primary); border: 1px solid var(--sf-border); font-family: var(--sf-font-ui); border-radius: var(--sf-radius-md);',
		error: {
			style: 'background: var(--sf-bg-surface); color: var(--sf-status-error); border: 1px solid var(--sf-status-error); font-family: var(--sf-font-ui); border-radius: var(--sf-radius-md);'
		},
		success: {
			style: 'background: var(--sf-bg-surface); color: var(--sf-status-online); border: 1px solid var(--sf-status-online); font-family: var(--sf-font-ui); border-radius: var(--sf-radius-md);'
		}
	}}
/>

	<div class="app-parent">
		<aside class="sidebar-container">
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
						isActive={$page.url.pathname === menu.href}
					/>
				{/each}
			</nav>
		</aside>

		<div class="app-layout">
			{#if showTabBar}
				<SessionTabBar />
			{/if}
			<slot />
		</div>
	</div>

<style>
	.app-parent {
		display: flex;
		flex-direction: row;
		height: 100vh;
		padding-top: 32px;
		overflow: hidden;
	}

	.sidebar-container {
		width: var(--sf-sidebar-width);
		flex-shrink: 0;
		height: 100%;
		background-color: var(--sf-bg-sidebar);
		border-right: 1px solid var(--sf-border);
		display: flex;
		flex-direction: column;
		padding-top: var(--sf-space-xl);
		overflow-y: auto; /* scroll if nav overflows */
	}

	.app-layout {
		flex: 1; /* take all remaining width */
		min-width: 0; /* prevent flex overflow */
		height: 100%; /* fill app-parent height */
		display: flex;
		flex-direction: column; /* tabbar on top, content below */
		overflow: hidden;
	}

	.content {
		flex: 1; /* fill remaining height below tabbar */
		min-height: 0; /* CRITICAL: allows flex child to scroll */
		overflow-y: auto;
	}

	.brand-section {
		padding: 0 var(--sf-space-lg);
		margin-bottom: var(--sf-space-xl);
	}
	.brand-title {
		font-family: var(--sf-font-title);
		font-size: 1.25rem;
		font-weight: 800;
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
	}
	.nav-menu {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 0 8px;
	}
</style>
