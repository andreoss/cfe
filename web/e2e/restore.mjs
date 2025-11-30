import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { promoteViaRoot, publishTopic } from './support.mjs'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const apiUrl = process.env.API_URL ?? 'http://127.0.0.1:58080'
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

async function mainText(driver) {
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      return (await driver.findElement(By.css('main')).getText()).replace(/\s+/g, ' ')
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function clickWhenReady(driver, locator, message) {
  await driver.wait(until.elementLocated(locator), 10000, message)
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      await driver.findElement(locator).click()
      return
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 10000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function scoreOf(username) {
  const response = await fetch(`${apiUrl}/api/users/${encodeURIComponent(username)}`)
  assert(response.ok, `reading ${username} failed with ${response.status}`)
  return (await response.json()).score
}

async function run() {
  const mod = await buildDriver()
  const author = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_rest_m_${suffix}`
  const authorName = `e2e_rest_a_${suffix}`
  const title = `A subject to restore ${suffix}`
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await register(author, authorName)

    await author.get(`${baseUrl}/s/general`)
    await author.wait(
      until.elementLocated(By.xpath("//button[text()='New topic']")),
      10000,
      'the new topic control should appear once posting is known to be allowed',
    )
    await author.findElement(By.xpath("//button[text()='New topic']")).click()
    await author.wait(until.elementLocated(By.name('title')), 10000)
    await author.findElement(By.name('title')).sendKeys(title)
    await author.findElement(By.name('body')).sendKeys('A body for the restore spec.')
    await clickWhenReady(author, By.xpath("//button[text()='Post']"), 'the post control')
    await publishTopic(title)
    await author.get(`${baseUrl}/s/general`)
    await author.wait(until.elementLocated(By.linkText(title)), 10000)
    await author.findElement(By.linkText(title)).click()
    await author.wait(until.elementLocated(By.name('comment-body')), 10000)
    const topicUrl = await author.getCurrentUrl()

    assert((await scoreOf(authorName)) === 0, 'the author should start at nothing')

    await mod.get(topicUrl)
    const noRestoreYet = await mod.findElements(By.xpath("//button[normalize-space(text())='Restore topic']"))
    assert(
      noRestoreYet.length === 0,
      'nothing that is not deleted should offer a restore control',
    )

    await clickWhenReady(mod, By.xpath("//button[text()='Delete topic']"), 'the delete control')
    const reason = await mod.wait(until.elementLocated(By.name('delete-reason')), 10000)
    await reason.sendKeys('spam')
    const choice = await mod.wait(until.elementLocated(By.name('penalty')), 10000)
    await choice.findElement(By.css("option[value='-30']")).click()
    await clickWhenReady(mod, By.xpath("//button[text()='Confirm delete']"), 'the confirm control')
    await mod.wait(
      async () => (await scoreOf(authorName)) === -30,
      10000,
      'deleting should cost the author the penalty that was chosen',
    )

    await mod.get(topicUrl)
    await mod.wait(
      until.elementLocated(By.xpath("//button[normalize-space(text())='Restore topic']")),
      10000,
      'a moderator should be offered a restore control on something deleted',
    )

    await author.get(topicUrl)
    await author.wait(until.elementLocated(By.css('main')), 10000)
    const authorSees = await mainText(author)
    assert(
      (await author.findElements(By.xpath("//button[normalize-space(text())='Restore topic']"))).length === 0,
      `a plain user must not be offered a restore control, saw: ${authorSees}`,
    )

    await clickWhenReady(mod, By.xpath("//button[normalize-space(text())='Restore topic']"), 'the restore control')
    await mod.wait(
      async () => (await scoreOf(authorName)) === 0,
      10000,
      'restoring should give back exactly what the deletion took',
    )
    await mod.wait(
      async () =>
        (await mod.findElements(By.xpath("//button[normalize-space(text())='Restore topic']"))).length === 0,
      10000,
      'once restored the control should retire without a reload',
    )

    await mod.get(`${baseUrl}/s/general`)
    await mod.wait(
      until.elementLocated(By.linkText(title)),
      10000,
      'a restored subject should be listed again',
    )

    await author.get(topicUrl)
    await author.wait(until.elementLocated(By.name('comment-body')), 10000)
    await author.findElement(By.name('comment-body')).sendKeys('A comment to restore')
    await clickWhenReady(author, By.xpath("//button[text()='Post comment']"), 'the comment control')
    await author.wait(
      until.elementLocated(By.xpath("//p[contains(., 'A comment to restore')]")),
      10000,
    )

    await mod.get(topicUrl)
    await clickWhenReady(mod, By.xpath("//button[normalize-space(text())='Delete comment']"), 'the delete control')
    const commentReason = await mod.wait(until.elementLocated(By.name('delete-reason')), 10000)
    await commentReason.sendKeys('spam')
    const commentChoice = await mod.wait(until.elementLocated(By.name('penalty')), 10000)
    await commentChoice.findElement(By.css("option[value='-20']")).click()
    await clickWhenReady(mod, By.xpath("//button[text()='Confirm delete']"), 'the confirm control')
    await mod.wait(
      async () => (await scoreOf(authorName)) === -20,
      10000,
      'deleting a comment should cost the author too',
    )

    await mod.get(topicUrl)
    await clickWhenReady(mod, By.xpath("//button[normalize-space(text())='Restore comment']"), 'the restore control')
    await mod.wait(
      async () => (await scoreOf(authorName)) === 0,
      10000,
      'restoring a comment should give back what it took',
    )

    console.log('e2e: restore flow passed')
  } finally {
    await mod.quit()
    await author.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
