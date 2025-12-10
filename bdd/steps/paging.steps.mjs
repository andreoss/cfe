import assert from 'node:assert/strict'
import { When, Then } from '@cucumber/cucumber'

When('they add enough remarks to fill more than one page', async function () {
  this.pageSize = await this.board.remarksPerPage()
  this.stamp = Date.now().toString(36)
  this.written = []
  for (let n = 0; n < this.pageSize + 2; n += 1) {
    const text = `Remark ${n} of many ${this.stamp}`
    const put = await this.board.addRemark(text)
    assert.equal(put.accepted, true, `remark ${n} should be taken`)
    this.written.push(text)
  }
})

Then('the first page does not hold every one of them', async function () {
  this.firstPage = await this.board.remarksOnPage(1)
  assert.ok(this.firstPage.length > 0, 'the first page should hold something')
  const missing = this.written.filter(
    (one) => !this.firstPage.some((seen) => seen.includes(one)),
  )
  assert.ok(
    missing.length > 0,
    'more remarks than a page holds should not all be on the first page',
  )
})

Then('a later page holds remarks the first page did not', async function () {
  const later = await this.board.remarksOnPage(2)
  assert.ok(later.length > 0, 'a later page should hold something')
  const onFirst = this.firstPage.join('\n')
  assert.ok(
    later.some((one) => !onFirst.includes(one)),
    'a later page should hold remarks the first page did not',
  )
})
