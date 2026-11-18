import { Controller, Delete, Get, Head, HttpCode, Options, Patch, Post, Put, BadRequestException, Body, Req } from '@nestjs/common';
import { execute_1 } from './generated';
import { KruntRuntimeService } from './krunt-runtime';

import { validateUser } from './models/User.model';

import type { User } from './models/User.model';


@Controller()
export class AppController {
  constructor(private readonly runtime: KruntRuntimeService) {}

  // krunt:source=node:10
  @HttpCode(200)
  @Post('/users')
  async handle_1(@Body() request: User): Promise<Array<User>> {
    let validatedRequest: User;
    try {
      validatedRequest = validateUser(request);
    } catch (error) {
      throw new BadRequestException(error instanceof Error ? error.message : 'Invalid request body');
    }
    return await execute_1(validatedRequest, this.runtime);
  }
}
