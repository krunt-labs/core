// krunt:source=node:50
import type { User } from '../models/User.model';
import type { KruntRepository, KruntRepositoryOperation } from '../krunt-runtime';

export class UserRepositoryRepository implements KruntRepository {
  private readonly records: Array<User> = [];

  async execute(kind: KruntRepositoryOperation, name: string, value: unknown): Promise<unknown> {
    switch (`${kind}:${name}`) {
      case 'QUERY:ListUsers': return this.listUsers();
      default: throw new Error(`Unknown repository operation ${name}`);
    }
  }

  // krunt:source=node:20
  async listUsers(): Promise<Array<User>> {
    return this.records as Array<User>;
  }

}
