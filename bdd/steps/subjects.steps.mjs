import assert from 'node:assert/strict'
import { When, Then } from '@cucumber/cucumber'

When('they change what the subject says', async function () {
  this.newBody = `The subject, said differently ${Date.now().toString(36)}`
  this.subjectEdit = await this.board.changeSubject(this.newBody)
})

Then('the subject reads differently now', async function () {
  assert.equal(this.subjectEdit.accepted, true, 'the board should accept the change')
})

When('they start a subject with a tag on it', async function () {
  this.tag = await this.board.tagForSubjects()
  this.started = await this.board.startSubjectTagged(this.tag)
})

Then('the subject carries that tag', async function () {
  const carried = await this.board.tagsOnSubject()
  assert.ok(
    carried.includes(this.tag),
    `the subject should carry ${this.tag}, carried ${carried.join(', ') || 'nothing'}`,
  )
})

When('they answer that answer', async function () {
  this.deeper = `An answer to the answer ${Date.now().toString(36)}`
  this.deepAnswer = await this.board.replyToRemark(this.answer, this.deeper)
})

Then('all three are on the subject', async function () {
  assert.equal(this.deepAnswer.accepted, true, 'the board should take the deeper answer')
  for (const text of [this.remark, this.answer, this.deeper]) {
    const found = await this.board.remarksOn(text)
    assert.equal(found.present, true, `"${text}" should be on the subject`)
  }
})

Then('the last one sits under the one before it', async function () {
  const under = await this.board.whatItAnswers(this.deeper)
  assert.equal(under.answers, true, 'the deeper answer should be held under the one it answers')
})
