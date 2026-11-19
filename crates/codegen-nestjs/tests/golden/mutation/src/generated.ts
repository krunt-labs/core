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
  // krunt:source=node:1
  // krunt:operation=1 ENTRY backend.http_route
  // krunt:types= -> User
  const operation_1 = request;
  // krunt:source=node:2
  // krunt:operation=2 OPERATION backend.mutation
  // krunt:types=User -> User
  const operation_2 = await runtime.mutation<User, User>("CreateUser", "UserRepository", operation_1);
  // krunt:source=node:3
  // krunt:operation=3 RETURN
  // krunt:types=User ->
  return operation_2;
}
