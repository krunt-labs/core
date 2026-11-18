import type { User } from './models/User.model';

export interface KruntRuntime {
  httpRequest(method: string, url: string, request: unknown): Promise<unknown>;
  query<TOutput>(name: string, repository: string): Promise<TOutput>;
  mutation<TInput, TOutput>(name: string, repository: string, value: TInput): Promise<TOutput>;
  log(level: string, message: unknown): void;
  storageRead(key: string): Promise<unknown>;
  storageWrite(key: string, value: unknown): Promise<unknown>;
}

export async function execute_1(request: User, runtime: KruntRuntime): Promise<Array<User>> {
  // krunt:source=node:1
  // krunt:operation=1 ENTRY backend.http_route
  // krunt:types= -> HttpRequest
  const operation_1 = request;
  // krunt:source=node:2
  // krunt:operation=2 OPERATION backend.query
  // krunt:types= -> Array<User>
  const operation_2 = await runtime.query<Array<User>>("ListUsers", "UserRepository");
  // krunt:source=node:3
  // krunt:operation=3 OPERATION backend.transform
  // krunt:types=Array<User> -> Array<User>
  const operation_3 = operation_2;
  // krunt:source=node:4
  // krunt:operation=4 RETURN
  // krunt:types=Array<User> ->
  return operation_3;
}
