import assert from 'node:assert/strict'
import { When, Then } from '@cucumber/cucumber'

When('they answer their own remark', async function () {
  this.answer = `An answer to that remark ${Date.now().toString(36)}`
  this.answered = await this.board.replyToRemark(this.remark, this.answer)
})

Then('the answer is on the subject', async function () {
  assert.equal(this.answered.accepted, true, 'the board should take the answer')
  this.found = await this.board.remarksOn(this.answer)
  assert.equal(this.found.present, true, 'the answer should be on the subject afterwards')
})

Then('the answer sits under the remark it answers', async function () {
  const under = await this.board.whatItAnswers(this.answer)
  assert.equal(
    under.answers,
    true,
    'an answer should be held under the remark it replies to, not on its own',
  )
})

When('they change what the remark says', async function () {
  this.changed = `The same remark, said differently ${Date.now().toString(36)}`
  this.edit = await this.board.changeRemark(this.remark, this.changed)
})

Then('the subject shows the changed words', async function () {
  assert.equal(this.edit.accepted, true, 'the board should accept the change')
  const found = await this.board.remarksOn(this.changed)
  assert.equal(found.present, true, 'the changed words should be on the subject')
})

Then('the subject no longer shows the words that were replaced', async function () {
  const gone = await this.board.remarksOn(this.remark)
  assert.equal(gone.present, false, 'the words that were replaced should be gone')
})

When('they take the remark away', async function () {
  this.removal = await this.board.removeRemark(this.remark)
})

Then('the remark is no longer offered on the subject', async function () {
  assert.equal(this.removal.accepted, true, 'the board should accept the removal')
  const gone = await this.board.remarksOn(this.remark)
  assert.equal(gone.present, false, 'a remark taken away should not be offered')
})

When('they start a subject', async function () {
  this.started = await this.board.startSubject()
})

Then('that subject is offered first in its section', async function () {
  const first = await this.board.firstSubjectInSection()
  assert.equal(
    first,
    this.started,
    `the newest subject should be offered first, but the first was ${first}`,
  )
})
