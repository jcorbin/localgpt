import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  tutorialSidebar: [
    'intro',
    {
      type: 'category',
      label: 'Getting Started',
      items: ['installation', 'quick-start'],
    },
    {
      type: 'category',
      label: 'CLI Commands',
      items: ['cli-commands', 'cli-chat', 'cli-ask', 'cli-daemon', 'cli-memory'],
    },
    {
      type: 'category',
      label: 'Core Features',
      items: ['memory-system', 'heartbeat', 'tools'],
    },
    {
      type: 'category',
      label: 'Reference',
      items: ['configuration', 'http-api'],
    },
  ],
};

export default sidebars;
