import { browser } from '$app/environment';
import { invoke } from '@tauri-apps/api/core';
import { redirect } from '@sveltejs/kit';
import { listen } from '@tauri-apps/api/event';

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

    const unlisten = await listen('oauth-success', (event) => {
        console.log('OAuth success');
        console.log(event.payload);
    });


    return {};
};
