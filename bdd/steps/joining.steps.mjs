import assert from 'node:assert/strict'
import { When, Then } from '@cucumber/cucumber'

When('somebody makes a new account', async function () {
  this.newcomer = `bddjoiner${Date.now().toString(36)}`
  this.joining = await this.board.makeAccount(this.newcomer)
})

Then('the account can be looked up by its name', async function () {
  assert.equal(
    this.joining.accepted,
    true,
    `the board should take a new account: ${this.joining.why ?? ''}`,
  )
  const found = await this.board.lookUpByName(this.newcomer)
  assert.equal(found.found, true, `the new account ${this.newcomer} should be found afterwards`)
})
