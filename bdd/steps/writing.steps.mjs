import assert from 'node:assert/strict'
import { Given, When, Then } from '@cucumber/cucumber'

Given('there is a subject to read', async function () {
  await this.board.ensureSubject()
  await this.board.subjectToWriteOn()
})

When('they add a remark to the subject', async function () {
  this.remark = `A remark left while comparing ${Date.now().toString(36)}`
  this.answer = await this.board.addRemark(this.remark)
})

When('somebody who has not signed in tries to add a remark', async function () {
  this.remark = `A remark from nobody ${Date.now().toString(36)}`
  this.answer = await this.board.addRemark(this.remark)
})

Then('the remark is on the subject', async function () {
  assert.equal(this.answer.accepted, true, 'the board should take the remark')
  this.found = await this.board.remarksOn(this.remark)
  assert.equal(this.found.present, true, 'the remark should be on the subject afterwards')
})

Then('the remark is under their name', function () {
  assert.equal(
    this.found.author,
    this.signedInAs,
    `the remark should be under ${this.signedInAs}, was under ${this.found.author}`,
  )
})

Then('the remark is refused', async function () {
  assert.equal(this.answer.accepted, false, 'a board should not take a remark from nobody')
  const found = await this.board.remarksOn(this.remark)
  assert.equal(found.present, false, 'a refused remark should not be on the subject')
})
