import clsx from 'clsx';
import Heading from '@theme/Heading';
import styles from './styles.module.css';

type FeatureItem = {
  title: string;
  description: JSX.Element;
};

const FeatureList: FeatureItem[] = [
  {
    title: 'Local & Private',
    description: (
      <>
        Single Rust binary, ~27MB. All data stays on your machine — markdown
        files, SQLite indexes, and local embeddings. No cloud storage, no
        telemetry. Just <code>cargo install localgpt</code>.
      </>
    ),
  },
  {
    title: 'Hybrid Memory Search',
    description: (
      <>
        Persistent markdown-based memory with hybrid search — SQLite FTS5 for
        keyword matching and local vector embeddings (fastembed) for semantic
        search. Your AI remembers and finds context across sessions.
      </>
    ),
  },
  {
    title: 'Desktop, Web & CLI',
    description: (
      <>
        Three ways to interact: a native desktop GUI (egui), an embedded web
        UI served from the binary, and a full-featured CLI with readline
        support. Plus an HTTP API and WebSocket for integrations.
      </>
    ),
  },
  {
    title: 'Autonomous Heartbeat',
    description: (
      <>
        Run LocalGPT as a daemon and it checks HEARTBEAT.md on a schedule —
        executing tasks, organizing knowledge, and reminding you of deadlines,
        all while you're away.
      </>
    ),
  },
  {
    title: 'Multi-Provider LLMs',
    description: (
      <>
        Works with Claude CLI, Anthropic API, OpenAI, and local Ollama
        models. Switch providers seamlessly while keeping your memory and
        conversation history intact.
      </>
    ),
  },
  {
    title: 'Security Built In',
    description: (
      <>
        Prompt injection defenses with a dedicated sanitization module,
        tool approval mode for dangerous operations, content delimiters,
        and workspace locking for safe concurrent access.
      </>
    ),
  },
];

function Feature({title, description}: FeatureItem) {
  return (
    <div className={clsx('col col--4')}>
      <div className="text--center padding-horiz--md">
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures(): JSX.Element {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
