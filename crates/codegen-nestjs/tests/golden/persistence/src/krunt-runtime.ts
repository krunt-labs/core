import type { KruntRuntime } from './generated';
import { UserRepositoryRepository } from './repositories/UserRepositoryRepository.repository';

export type KruntRepositoryOperation = 'QUERY' | 'MUTATION';

export interface KruntRepository {
  execute(kind: KruntRepositoryOperation, name: string, value: unknown): Promise<unknown>;
}

export class KruntRuntimeService implements KruntRuntime {
  private readonly storage = new Map<string, unknown>();

  constructor(
    private readonly userRepositoryRepository: UserRepositoryRepository,
  ) {}

  private repository(name: string): KruntRepository | undefined {
    switch (name) {
      case 'UserRepository': return this.userRepositoryRepository;
      default: return undefined;
    }
  }

  async httpRequest(method: string, url: string, request: unknown): Promise<unknown> {
    const response = await fetch(url, {
      method,
      body: request === undefined ? undefined : JSON.stringify(request),
      headers: request === undefined ? undefined : { 'content-type': 'application/json' },
    });
    const text = await response.text();
    if (text.length === 0) {
      return undefined;
    }
    try {
      return JSON.parse(text) as unknown;
    } catch {
      return text;
    }
  }

  async query<TOutput>(name: string, repository: string): Promise<TOutput> {
    const adapter = this.repository(repository);
    if (adapter === undefined) {
      throw new Error(`No repository provider configured for query ${repository}.${name}`);
    }
    return adapter.execute('QUERY', name, undefined) as Promise<TOutput>;
  }

  async mutation<TInput, TOutput>(name: string, repository: string, value: TInput): Promise<TOutput> {
    const adapter = this.repository(repository);
    if (adapter === undefined) {
      throw new Error(`No repository provider configured for mutation ${repository}.${name}`);
    }
    return adapter.execute('MUTATION', name, value) as Promise<TOutput>;
  }

  log(level: string, message: unknown): void {
    switch (level) {
      case 'ERROR':
        console.error(message);
        break;
      case 'WARN':
        console.warn(message);
        break;
      case 'DEBUG':
        console.debug(message);
        break;
      default:
        console.info(message);
    }
  }

  async storageRead(key: string): Promise<unknown> {
    return this.storage.get(key);
  }

  async storageWrite(key: string, value: unknown): Promise<unknown> {
    this.storage.set(key, value);
    return value;
  }
}
