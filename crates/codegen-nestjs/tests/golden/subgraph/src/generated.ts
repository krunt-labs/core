import type { User } from './models/User.model';

export interface KruntRuntime {
  httpRequest(method: string, url: string, request: unknown): Promise<unknown>;
  query<TOutput>(name: string, repository: string): Promise<TOutput>;
  mutation<TInput, TOutput>(name: string, repository: string, value: TInput): Promise<TOutput>;
  log(level: string, message: unknown): void;
  storageRead(key: string): Promise<unknown>;
  storageWrite(key: string, value: unknown): Promise<unknown>;
}

export async function execute_1(request: User, runtime: KruntRuntime): Promise<User> {
  let operation_1: User = undefined as unknown as User;
  let control: number | null = 1;
  while (control !== null) {
    switch (control) {
      case 1: {
        // krunt:source=node:1
        // krunt:operation=1 ENTRY backend.http_route
        // krunt:types= -> User
        operation_1 = request;
        control = 2;
        break;
      }
      case 2: {
        // krunt:source=node:2
        // krunt:operation=2 SUBGRAPH 40
        // krunt:subgraph=40
        await subgraph_40(runtime);
        control = 3;
        break;
      }
      case 3: {
        // krunt:source=node:3
        // krunt:operation=3 RETURN
        // krunt:types=User ->
        return undefined as never;
      }
      default: {
        control = null;
        break;
      }
    }
  }
  return undefined as never;
}

export async function subgraph_40(runtime: KruntRuntime): Promise<void> {
  // krunt:source=node:6
  // krunt:operation=6 OPERATION backend.query
  // krunt:types= -> Array<User>
  const operation_6 = await runtime.query<Array<User>>("ListUsers", "UserRepository");
}
