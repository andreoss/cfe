import assert from 'node:assert/strict'
import { When, Then } from '@cucumber/cucumber'

When('they start a subject carrying an unusual word', async function () {
  this.word = `zarquon${Date.now().toString(36)}`
  this.started = await this.board.startSubjectCarrying(this.word)
})

Then('the subject can be found by that word', async function () {
  const found = await this.board.findByWord(this.word)
  assert.ok(
    found.count > 0,
    `searching for ${this.word} should find something, found ${found.count}`,
  )
  assert.ok(
    found.titles.some((one) => one.includes(this.word)),
    `the subject should be among what was found, saw: ${found.titles.join(' / ')}`,
  )
})
