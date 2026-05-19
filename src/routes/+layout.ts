// Tauri doesn't have a Node.js server to do proper SSR
// Tauri doesn't have a Node.js server to do proper SSR
// so we will use adapter-static to prerender the app (SSG)
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
// Tauri doesn't have a Node.js server to do proper SSR
// so we will use adapter-static to prerender the app (SSG)
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import { browser } from '$app/environment';
import { invoke } from '@tauri-apps/api/core';
import { redirect } from '@sveltejs/kit';

export const prerender = true;
export const ssr = false;

export const load = async ({ url }: { url: { pathname: string } }) => {
    if (browser) {
        try {
            const isAuthenticated = await invoke<boolean>('oauth_is_authenticated');

            if (!isAuthenticated && url.pathname !== '/login') {
                throw redirect(302, '/login');
            }
        } catch (err) {
            if (url.pathname !== '/login') {
                throw redirect(302, '/login');
            }
        }
    }
    return {};
};
