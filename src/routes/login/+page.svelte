<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { listen } from '@tauri-apps/api/event';
    import { open } from '@tauri-apps/plugin-shell';
    import { goto } from '$app/navigation';
    import { ShieldCheck, LogIn } from '@lucide/svelte';

    let port: number;
    let loading = $state(false);

    async function startOAuthFlow() {
        try {
            loading = true;


            await invoke('start_server');
            
            const authUrl: string = await invoke('open_oauth_login');
            console.log('Opening browser to:', authUrl);
            await open(authUrl);

            const unlisten = await listen<string>('oauth_success', () => {
                console.log('OAuth flow completed successfully');
                localStorage.setItem('auth', 'true'); // Required for the layout guard
                unlisten();
                goto('/'); 
            });

        } catch (error) {
            console.error('Error during OAuth flow:', error);
            loading = false;
        }
    }
</script>

<main class="login-container">
    <div class="login-card">
        <div class="brand-section">
            <ShieldCheck size={48} color="var(--sf-accent)" strokeWidth={1.5} />
            <h1 class="brand-title">SNOWFLAKES</h1>
            <p class="brand-subtitle">SECURE SSH MANAGER</p>
        </div>

        <div class="login-content">
            <p class="login-description">
                Welcome back. Please authenticate with your SLC account to access your environments.
            </p>
            
            <button class="login-button" onclick={startOAuthFlow} disabled={loading}>
                {#if loading}
                    <span class="spinner"></span>
                    Authenticating...
                {:else}
                    <LogIn size={20} />
                    Login with SLC
                {/if}
            </button>
        </div>
    </div>
</main>

<style>
    .login-container {
        width: 100%;
        height: 100vh;
        background-color: var(--sf-bg-app);
        display: flex;
        align-items: center;
        justify-content: center;
        font-family: var(--sf-font-ui);
    }

    .login-card {
        background-color: var(--sf-bg-surface);
        border: 1px solid var(--sf-border);
        border-radius: var(--sf-radius-lg);
        width: 400px;
        padding: var(--sf-space-xl);
        display: flex;
        flex-direction: column;
        align-items: center;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    }

    .brand-section {
        display: flex;
        flex-direction: column;
        align-items: center;
        margin-bottom: var(--sf-space-xl);
        text-align: center;
    }

    .brand-title {
        font-family: var(--sf-font-title);
        font-size: 2rem;
        font-weight: 800;
        color: var(--sf-text-primary);
        margin: var(--sf-space-sm) 0 0 0;
        letter-spacing: 1px;
    }

    .brand-subtitle {
        font-size: 0.8rem;
        font-weight: 700;
        color: var(--sf-accent);
        margin: 4px 0 0 0;
        text-transform: uppercase;
        letter-spacing: 2px;
    }

    .login-content {
        width: 100%;
        display: flex;
        flex-direction: column;
        gap: var(--sf-space-lg);
    }

    .login-description {
        color: var(--sf-text-secondary);
        font-size: 0.95rem;
        text-align: center;
        line-height: 1.5;
        margin: 0;
    }

    .login-button {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: var(--sf-space-sm);
        background-color: var(--sf-accent);
        color: var(--sf-text-on-accent);
        border: none;
        border-radius: var(--sf-radius-md);
        padding: var(--sf-space-md) var(--sf-space-lg);
        font-size: 1rem;
        font-weight: 600;
        font-family: var(--sf-font-ui);
        cursor: pointer;
        transition: all 0.2s ease;
        width: 100%;
    }

    .login-button:hover:not(:disabled) {
        background-color: var(--sf-accent-hover);
        transform: translateY(-1px);
    }

    .login-button:disabled {
        opacity: 0.7;
        cursor: not-allowed;
    }

    .spinner {
        width: 18px;
        height: 18px;
        border: 2px solid rgba(4, 44, 83, 0.3);
        border-radius: 50%;
        border-top-color: var(--sf-text-on-accent);
        animation: spin 1s linear infinite;
    }

    @keyframes spin {
        to { transform: rotate(360deg); }
    }
</style>