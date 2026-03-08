import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://fsecada01.github.io',
  base: '/bus_channel_strip',
  integrations: [
    starlight({
      title: 'Bus Channel Strip',
      description: 'Professional 6-module bus processor — VST3 & CLAP',
      social: {
        github: 'https://github.com/fsecada01/bus_channel_strip',
      },
      customCss: ['./src/styles/custom.css'],
      editLink: {
        baseUrl: 'https://github.com/fsecada01/bus_channel_strip/edit/main/site/',
      },
      lastUpdated: true,
      favicon: '/bus_channel_strip/favicon.svg',
      sidebar: [
        { label: 'Overview', link: '/' },
        { label: 'Installation', link: '/install/' },
        {
          label: 'Modules',
          items: [
            { label: 'Signal Chain', link: '/modules/' },
            { label: 'API5500 EQ', link: '/modules/api5500/' },
            { label: 'ButterComp2', link: '/modules/buttercomp2/' },
            { label: 'Pultec EQ', link: '/modules/pultec/' },
            { label: 'Dynamic EQ', link: '/modules/dynamic_eq/' },
            { label: 'Transformer', link: '/modules/transformer/' },
            { label: 'Punch', link: '/modules/punch/' },
          ],
        },
        {
          label: 'Presets & Techniques',
          items: [
            { label: 'Settings & Techniques', link: '/presets/techniques/' },
            { label: 'Genre Signal Chains', link: '/presets/genres/' },
            { label: 'Instrument Buses', link: '/presets/buses/' },
          ],
        },
        { label: 'Contributing', link: '/contributing/' },
        { label: 'Architecture', link: '/architecture/' },
        { label: 'Changelog', link: '/changelog/' },
        { label: 'Parameter Reference', link: '/parameters/' },
      ],
    }),
  ],
});
