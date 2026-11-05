import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightLlmsTxt from 'starlight-llms-txt';
import { sqlquackDark, sqlquackLight } from './src/code-themes.mjs';
import { redirects, sidebar } from './src/navigation.mjs';
import { figuresIntegration } from './src/figures.mjs';

export default defineConfig({
	// Placeholder: the public domain is not chosen yet.
	site: 'https://sqlquack.io',
	redirects,
	integrations: [
		figuresIntegration(),
		starlight({
			title: 'SQLQuack',
			description: 'A Snowflake SQL compiler: parse, check and translate to DuckDB, or refuse with a precise reason.',
			logo: {
				light: './src/assets/logo-light.png',
				dark: './src/assets/logo-dark.png',
				replacesTitle: true,
			},
			favicon: '/favicon.png',
			head: [
				{ tag: 'meta', attrs: { property: 'og:image', content: 'https://sqlquack.io/assets/og-sqlquack.png' } },
				{ tag: 'meta', attrs: { property: 'og:image:width', content: '1200' } },
				{ tag: 'meta', attrs: { property: 'og:image:height', content: '630' } },
				{ tag: 'meta', attrs: { name: 'twitter:image', content: 'https://sqlquack.io/assets/og-sqlquack.png' } },
			],
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/sqlstack-io/sqlquack' },
			],
			components: {
				PageFrame: './src/components/PageFrame.astro',
				TwoColumnContent: './src/components/TwoColumnContent.astro',
				Header: './src/components/Header.astro',
				PageTitle: './src/components/PageTitle.astro',
				SiteTitle: './src/components/SiteTitle.astro',
				Pagination: './src/components/Pagination.astro',
				ThemeSelect: './src/components/ThemeSelect.astro',
				Footer: './src/components/Footer.astro',
			},
			customCss: ['./src/styles/fonts.css', './src/styles/sqlquack.css'],
			expressiveCode: {
				themes: [sqlquackDark, sqlquackLight],
				defaultProps: {
					// Plain boxes for shell snippets, without the fake terminal title bar.
					overridesByLang: { 'bash,sh,shell,zsh,console,powershell': { frame: 'none' } },
				},
				styleOverrides: {
					borderRadius: '2px',
					borderWidth: '1px',
					codeFontFamily: 'var(--sl-font-mono)',
					codeFontSize: '13px',
					codeLineHeight: '1.65',
					uiFontFamily: 'var(--sl-font)',
					frames: {
						shadowColor: 'transparent',
						inlineButtonBorder: 'transparent',
						inlineButtonBackgroundIdleOpacity: '0',
						inlineButtonBackgroundHoverOrFocusOpacity: '0.08',
						inlineButtonForeground: 'var(--sqq-muted)',
						frameBoxShadowCssValue: 'none',
						editorTabBarBackground: 'var(--sqq-code-bg)',
						editorActiveTabBackground: 'var(--sqq-code-bg)',
						editorActiveTabIndicatorTopColor: 'var(--sqq-primary)',
						editorTabBarBorderBottomColor: 'var(--sqq-line)',
						terminalTitlebarBackground: 'var(--sqq-code-bg)',
						terminalTitlebarBorderBottomColor: 'var(--sqq-line)',
						terminalTitlebarDotsOpacity: '0.35',
						terminalBackground: 'var(--sqq-code-bg)',
					},
					borderColor: 'var(--sqq-line)',
				},
			},
			sidebar,
			plugins: [
				starlightLlmsTxt({
					projectName: 'SQLQuack',
					description:
						'SQLQuack parses Snowflake SQL, reports what Snowflake would reject with coded diagnostics, and converts it into DuckDB SQL or refuses it with a precise reason.',
				}),
			],
		}),
	],
});
