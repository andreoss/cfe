import assert from 'node:assert/strict'
import { When, Then } from '@cucumber/cucumber'

Then('it offers at least one section', async function () {
  const sections = await this.board.sections()
  assert.ok(sections.length > 0, 'a board should be divided into at least one place to post')
})

When('somebody who has not signed in opens a section', async function () {
  this.seen = await this.board.openSection()
})

Then('the section names itself', function () {
  assert.equal(this.seen.found, true, 'the section should open')
  assert.ok(this.seen.names.length > 0, 'the section should say what it is')
})

When('somebody who has not signed in opens a subject', async function () {
  await this.board.ensureSubject()
  this.seen = await this.board.openSubject()
})

Then('the subject shows its title', function () {
  assert.equal(this.seen.found, true, 'the subject should open')
  assert.ok(this.seen.title.length > 0, 'a subject should carry a title')
})

Then('the subject names who wrote it', function () {
  assert.ok(this.seen.author.length > 0, 'a subject should say who wrote it')
})

When('somebody looks up the account of an administrator', async function () {
  this.seen = await this.board.lookUpAccount('administrator')
})

Then('the account names itself', function () {
  assert.equal(this.seen.found, true, 'the account should be found')
  assert.ok(this.seen.names.length > 0, 'the account should name itself')
})

When('somebody opens a subject that does not exist', async function () {
  this.seen = await this.board.openMissingSubject()
})

Then('the board says there is nothing there', function () {
  assert.equal(this.seen.found, false, 'a subject that is not there should not be found')
})
