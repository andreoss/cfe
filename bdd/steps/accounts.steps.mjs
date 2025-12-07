import assert from 'node:assert/strict'
import { When, Then } from '@cucumber/cucumber'

When('somebody looks up an account that does not exist', async function () {
  this.seen = await this.board.lookUpMissingAccount()
})

Then('the account is not found', function () {
  assert.equal(this.seen.found, false, 'an account that is not there should not be found')
})

When('they look themselves up', async function () {
  this.seen = await this.board.lookUpAccount('administrator')
})

Then('the account found is the one they signed in as', function () {
  assert.equal(this.seen.found, true, 'the account should be found')
  assert.equal(
    this.seen.names,
    this.signedInAs,
    `the account should be ${this.signedInAs}, was ${this.seen.names}`,
  )
})

When('somebody looks up the account of a moderator', async function () {
  this.seen = await this.board.lookUpAccount('moderator')
})
