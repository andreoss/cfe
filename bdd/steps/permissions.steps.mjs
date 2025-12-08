import assert from 'node:assert/strict'
import { When, Then } from '@cucumber/cucumber'

When('a reader signs in instead', async function () {
  await this.board.signOut()
  this.signedInAs = await this.board.signIn('reader')
})

When('that reader tries to take the remark away', async function () {
  this.attempt = await this.board.removeRemark(this.remark)
})

When('that reader tries to change the remark', async function () {
  this.attempt = await this.board.changeRemark(this.remark, 'words put there by somebody else')
})

Then('the attempt is refused', function () {
  assert.equal(this.attempt.accepted, false, 'the board should refuse the attempt')
})

Then('the remark is still on the subject', async function () {
  const found = await this.board.remarksOn(this.remark)
  assert.equal(found.present, true, 'the remark should still be there afterwards')
})
