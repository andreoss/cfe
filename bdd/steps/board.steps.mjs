import assert from 'node:assert/strict'
import { Given, When, Then, Before, setDefaultTimeout } from '@cucumber/cucumber'
import { openBoard } from '../bindings/board.mjs'

setDefaultTimeout(60000)

Before(async function () {
  this.board = await openBoard()
  this.signedInAs = null
})

Given('the board is answering', async function () {
  assert.equal(await this.board.answering(), true, 'the board should answer at all')
})

When('an administrator signs in', async function () {
  this.signedInAs = await this.board.signIn('administrator')
})

When('a reader signs in', async function () {
  this.signedInAs = await this.board.signIn('reader')
})

When('a reader signs in with the wrong password', async function () {
  this.signedInAs = null
  await this.board.signIn('reader', 'not-the-right-password')
})

When('they sign out', async function () {
  await this.board.signOut()
  this.signedInAs = null
})

Then('the board knows who they are', async function () {
  const who = await this.board.whoAmI()
  assert.equal(
    who,
    this.signedInAs,
    `the board should know them as ${this.signedInAs}, said ${who}`,
  )
})

Then('the board does not know who they are', async function () {
  const who = await this.board.whoAmI()
  assert.equal(who, null, `nobody should be signed in, but the board said ${who}`)
})
