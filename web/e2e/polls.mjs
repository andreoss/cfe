import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { publishTopic } from './support.mjs'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const chromedriverPath = process.env.CHROMEDRIVER_PATH

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function buildDriver() {
  const options = new chrome.Options()
  options.addArguments('--headless=new', '--no-sandbox', '--disable-gpu')
  const builder = new Builder().forBrowser('chrome').setChromeOptions(options)
  if (chromedriverPath) {
    const service = new chrome.ServiceBuilder(chromedriverPath)
    builder.setChromeService(service)
  }
  return builder.build()
}

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 5000)
}

function optionButton(text) {
  return By.xpath(`//button[starts-with(normalize-space(.), '${text} (')]`)
}

async function optionText(driver, text) {
  const el = await driver.findElement(optionButton(text))
  return (await el.getText()).trim()
}

async function waitForOption(driver, text, expected) {
  await driver.wait(async () => (await optionText(driver, text)) === expected, 5000)
}

async function run() {
  const author = await buildDriver()
  const suffix = Date.now().toString(36)
  const authorName = `e2e_poll_a_${suffix}`
  const voterName = `e2e_poll_b_${suffix}`
  const topicTitle = `Pollable subject ${suffix}`
  let voter
  let anon
  try {
    await register(author, authorName)

    await author.get(`${baseUrl}/s/general`)
    await author.wait(until.elementLocated(By.xpath("//button[text()='New topic']")), 5000)
    await author.wait(
      until.elementLocated(By.xpath("//button[text()='New topic']")),
      10000,
      'the new topic control should appear once posting is known to be allowed',
    )
    await author.findElement(By.xpath("//button[text()='New topic']")).click()
    await author.wait(until.elementLocated(By.name('title')), 10000)
    await author.findElement(By.name('title')).sendKeys(topicTitle)
    await author.findElement(By.name('body')).sendKeys('Body for the polls e2e spec.')
    await author.findElement(By.xpath("//button[text()='Post']")).click()
    await publishTopic(topicTitle)
    await author.get(`${baseUrl}/s/general`)
    await author.wait(until.elementLocated(By.linkText(topicTitle)), 5000)
    await author.findElement(By.linkText(topicTitle)).click()
    await author.wait(until.elementLocated(By.name('comment-body')), 5000)
    const topicUrl = await author.getCurrentUrl()

    let text = await author.findElement(By.css('main')).getText()
    assert(!text.includes('Total votes:'), 'a new topic should have no poll')

    await author
      .wait(until.elementLocated(By.xpath("//button[normalize-space(.)='Add poll']")), 5000)
      .then((b) => b.click())
    const question = await author.wait(until.elementLocated(By.name('poll-question')), 5000)
    await question.sendKeys('Which one?')
    await author.findElement(By.name('poll-option-1')).sendKeys('Alpha')
    await author.findElement(By.name('poll-option-2')).sendKeys('Beta')
    await author.findElement(By.name('poll-option-3')).sendKeys('Gamma')
    await author.findElement(By.xpath("//button[normalize-space(.)='Create poll']")).click()

    await author.wait(until.elementLocated(optionButton('Alpha')), 5000)
    await waitForOption(author, 'Alpha', 'Alpha (0)')
    await waitForOption(author, 'Gamma', 'Gamma (0)')
    text = await author.findElement(By.css('main')).getText()
    assert(text.includes('Which one?'), 'the poll question should be shown')
    assert(text.includes('Total votes: 0'), 'a fresh poll should report zero votes')

    await author.findElement(optionButton('Alpha')).click()
    await waitForOption(author, 'Alpha', 'Alpha (1)')
    text = await author.findElement(By.css('main')).getText()
    assert(text.includes('Total votes: 1'), 'the total should follow the vote')
    let mine = await author.findElements(By.css('button.mine'))
    assert(mine.length > 0, 'your own choice should be marked')

    await author.navigate().refresh()
    await author.wait(until.elementLocated(optionButton('Alpha')), 5000)
    await waitForOption(author, 'Alpha', 'Alpha (1)')

    voter = await buildDriver()
    await register(voter, voterName)
    await voter.get(topicUrl)
    await voter.wait(until.elementLocated(optionButton('Beta')), 5000)
    text = await voter.findElement(By.css('main')).getText()
    assert(!text.includes('Add poll'), 'only the topic author may add a poll')
    await voter.findElement(optionButton('Beta')).click()
    await waitForOption(voter, 'Beta', 'Beta (1)')
    text = await voter.findElement(By.css('main')).getText()
    assert(text.includes('Total votes: 2'), 'a second voter should raise the total')

    await author.navigate().refresh()
    await author.wait(until.elementLocated(optionButton('Beta')), 5000)
    await author.findElement(optionButton('Beta')).click()
    await waitForOption(author, 'Beta', 'Beta (2)')
    await waitForOption(author, 'Alpha', 'Alpha (0)')
    text = await author.findElement(By.css('main')).getText()
    assert(
      text.includes('Total votes: 2'),
      'switching your vote must move it, not add another',
    )

    anon = await buildDriver()
    await anon.get(topicUrl)
    await anon.wait(until.elementLocated(optionButton('Beta')), 5000)
    await waitForOption(anon, 'Beta', 'Beta (2)')
    const anonButton = await anon.findElement(optionButton('Beta'))
    assert(
      (await anonButton.getAttribute('disabled')) !== null,
      'a signed-out visitor must not be able to vote',
    )

    console.log('e2e: polls flow passed')
  } finally {
    await author.quit()
    if (voter) await voter.quit()
    if (anon) await anon.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
