// krunt:source=node:5
import type { User } from '../models/User.model';
import type { KruntRepository, KruntRepositoryOperation } from '../krunt-runtime';

export class UserRepositoryRepository implements KruntRepository {
  private readonly records: Array<User> = [];

  async execute(kind: KruntRepositoryOperation, name: string, value: unknown): Promise<unknown> {
    switch (`${kind}:${name}`) {
      case 'MUTATION:CreateUser': return this.createUser(value as User);
      default: throw new Error(`Unknown repository operation ${name}`);
    }
  }

  // krunt:source=node:2
  async createUser(value: User): Promise<User> {
    const record = value as User;
    this.records.push(record);
    return record as User;
  }

}
