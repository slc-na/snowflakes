<script lang="ts">
	import { page } from "$app/stores";
	import { goto } from "$app/navigation";
	import {
		House,
		ShieldCheck,
		ArrowLeftRight,
		Settings,
		LayoutGrid,
		Clock,
		Sun,
		Moon
	} from "@lucide/svelte";
	import SidebarElement from "../components/home/SidebarElement.svelte";
	import "../layout.css";
	import TitleBar from "../components/TitleBar.svelte";
	import SessionTabBar from "../components/terminal/SessionTabBar.svelte";
	import { Toaster } from 'svelte-sonner';
	import { onMount } from "svelte";
	import { loadAppZoom, applyAppZoom, incrementAppZoom, resetAppZoom } from "$lib/appZoom";

	const menus = [
		{ icon: House, text: "HOME", href: "/" },
		{ icon: Clock, text: "RECENTS", href: "/recents" },
		{ icon: LayoutGrid, text: "MULTI WINDOW", href: "/window" },
		{ icon: Settings, text: "SETTINGS", href: "/settings" },
	];

	let theme = $state("dark");
	let appZoom = $state(1);

	onMount(() => {
		const stored = localStorage.getItem("sf-theme");
		if (stored === "light") {
			theme = "light";
			document.documentElement.setAttribute("data-theme", "light");
		}

		appZoom = loadAppZoom();
		applyAppZoom(appZoom);
	});

	function isSessionPage() {
		return $page.url.pathname.includes("/session");
	}

	function handleAppZoomKeydown(e: KeyboardEvent) {
		if (!e.ctrlKey || isSessionPage()) return;
		if (e.key === "=" || e.key === "+") {
			e.preventDefault();
			appZoom = incrementAppZoom(appZoom, 1);
		} else if (e.key === "-") {
			e.preventDefault();
			appZoom = incrementAppZoom(appZoom, -1);
		} else if (e.key === "0") {
			e.preventDefault();
			appZoom = resetAppZoom();
		}
	}

	const altShortcuts: Record<string, string> = {
		h: "/",
		r: "/recents",
		m: "/window",
		s: "/settings",
	};

	function isTerminalTarget(target: EventTarget | null): boolean {
		return target instanceof HTMLElement && !!target.closest(".xterm");
	}

	function handleAltShortcuts(e: KeyboardEvent) {
		if (!e.altKey || isTerminalTarget(e.target)) return;
		const href = altShortcuts[e.key.toLowerCase()];
		if (href) {
			e.preventDefault();
			goto(href);
		}
	}

	function toggleTheme() {
		if (theme === "dark") {
			theme = "light";
			document.documentElement.setAttribute("data-theme", "light");
			localStorage.setItem("sf-theme", "light");
		} else {
			theme = "dark";
			document.documentElement.removeAttribute("data-theme");
			localStorage.setItem("sf-theme", "dark");
		}
	}

	let showTabBar = $derived(menus.some(
		(menu) =>
			menu.href === $page.url.pathname ||
			$page.url.pathname.includes("/session") ||
			$page.url.pathname.includes("/files") ||
			$page.url.pathname.includes("/remote"),
	));
	
	let isLoginPage = $derived($page.url.pathname === '/login');
</script>

<svelte:window
	onkeydown={(e) => {
		handleAppZoomKeydown(e);
		handleAltShortcuts(e);
	}}
/>

<div id="zoom-root">
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

				<div class="sidebar-spacer"></div>

				<button class="theme-toggle" onclick={toggleTheme}>
					{#if theme === 'dark'}
					  <Sun size={16} /> <span class="toggle-text">LIGHT MODE</span>
					{:else}
					  <Moon size={16} /> <span class="toggle-text">DARK MODE</span>
					{/if}
				</button>
			</aside>

			<div class="app-layout">
				{#if showTabBar}
					<SessionTabBar />
				{/if}
				<slot />
			</div>
		</div>
</div>

<style>
	:global(html, body) {
		margin: 0;
		height: 100%;
		overflow: hidden;
	}

	#zoom-root {
		transform-origin: top left;
	}

	.app-parent {
		display: flex;
		flex-direction: row;
		height: 100%;
		box-sizing: border-box;
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

	.sidebar-spacer {
		flex: 1;
	}

	.theme-toggle {
		display: flex;
		align-items: center;
		gap: 12px;
		margin: 16px 8px;
		padding: 10px 14px;
		background: transparent;
		border: none;
		border-radius: var(--sf-radius-md);
		color: var(--sf-text-secondary);
		cursor: pointer;
		transition: all 0.2s;
	}

	.theme-toggle:hover {
		background: var(--sf-bg-hover);
		color: var(--sf-text-primary);
	}

	.toggle-text {
		font-family: var(--sf-font-ui);
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 1px;
	}
</style>
