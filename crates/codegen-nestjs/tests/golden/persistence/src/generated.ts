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
  // krunt:source=node:10
  // krunt:operation=1 ENTRY backend.http_route
  // krunt:types= -> Request
  const operation_1 = request;
  // krunt:source=node:20
  // krunt:operation=2 OPERATION backend.query
  // krunt:types= -> Array<User>
  const operation_2 = await runtime.query<Array<User>>("ListUsers", "UserRepository");
  // krunt:source=node:30
  // krunt:operation=3 RETURN
  // krunt:types=Array<User> -> 
  return operation_2;
}
