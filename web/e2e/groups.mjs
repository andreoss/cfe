import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { promoteViaRoot } from './support.mjs'

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

async function openNewTopic(driver) {
  let last
  for (let attempt = 0; attempt < 10; attempt += 1) {
    try {
      await clickWhenReady(
        driver,
        By.xpath("//button[text()='New topic']"),
        'the new topic control should be present',
      )
      await driver.wait(until.elementLocated(By.name('title')), 2000)
      return
    } catch (err) {
      last = err
    }
  }
  throw last
}

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function signIn(driver, username) {
  await driver.get(`${baseUrl}/sign-in`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function mainText(driver) {
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      return await driver.findElement(By.css('main')).getText()
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

async function run() {
  const mod = await buildDriver()
  const author = await buildDriver()
  const bystander = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_grp_m_${suffix}`
  const authorName = `e2e_grp_a_${suffix}`
  const otherName = `e2e_grp_o_${suffix}`
  const groupName = `Group ${suffix}`
  const groupSlug = `grp-${suffix}`
  const topicTitle = `Queued topic ${suffix}`
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await signIn(mod, modName)
    await register(author, authorName)
    await register(bystander, otherName)

    await mod.get(`${baseUrl}/s/general/groups`)
    await mod.wait(until.elementLocated(By.xpath("//button[text()='New group']")), 10000)
    await clickWhenReady(mod, By.xpath("//button[text()='New group']"), 'the new group control should be present')
    await mod.findElement(By.name('name')).sendKeys(groupName)
    await mod.findElement(By.name('slug')).sendKeys(groupSlug)
    await clickWhenReady(mod, By.xpath("//button[text()='Create']"), 'the create control should be present')
    await mod.wait(until.elementLocated(By.linkText(groupName)), 10000)

    await author.get(`${baseUrl}/s/general/groups`)
    await author.wait(until.elementLocated(By.linkText(groupName)), 10000)
    const noCreate = await author.findElements(By.xpath("//button[text()='New group']"))
    assert(noCreate.length === 0, 'a plain user must not be offered group creation')

    await author.get(`${baseUrl}/s/general`)
    await author.wait(until.elementLocated(By.xpath("//button[text()='New topic']")), 10000)
    await openNewTopic(author)
    await author.findElement(By.name('title')).sendKeys(topicTitle)
    await author.findElement(By.name('body')).sendKeys('This one waits for a moderator.')
    const picker = await author.findElement(By.name('group'))
    await picker.findElement(By.xpath(`./option[@value='${groupSlug}']`)).click()
    await clickWhenReady(author, By.xpath("//button[text()='Post']"), 'the post control should be present')

    await mod.get(`${baseUrl}/s/general`)
    await mod.wait(
      async () => (await mainText(mod)).includes(topicTitle),
      10000,
      'a moderator should see the queued topic in the section',
    )
    assert(
      (await mainText(mod)).includes('(pending)'),
      'a moderator should see the topic marked as pending',
    )

    await bystander.get(`${baseUrl}/s/general`)
    await bystander.wait(until.elementLocated(By.css('main')), 10000)
    assert(
      !(await mainText(bystander)).includes(topicTitle),
      'another user must not see a topic that is still in the queue',
    )

    await author.get(`${baseUrl}/s/general`)
    await author.wait(
      async () => (await mainText(author)).includes(topicTitle),
      10000,
      'the author should still see their own queued topic',
    )
    await clickWhenReady(author, By.linkText(topicTitle), 'the queued topic should be openable by its author')
    await author.wait(
      async () => (await mainText(author)).includes('Awaiting moderation.'),
      10000,
      'the author should be told their topic is awaiting moderation',
    )
    const topicUrl = await author.getCurrentUrl()

    await bystander.get(topicUrl)
    await bystander.wait(until.elementLocated(By.css('main')), 10000)
    assert(
      !(await mainText(bystander)).includes('This one waits for a moderator.'),
      'a queued topic must not be readable by direct link either',
    )

    await mod.get(topicUrl)
    await clickWhenReady(mod, By.xpath("//summary[text()='Premoderation']"), 'the premoderation panel should be present')
    await clickWhenReady(mod, By.xpath("//button[text()='Commit topic']"), 'a moderator should be offered a commit control')
    await mod.wait(
      async () => !(await mainText(mod)).includes('Awaiting moderation.'),
      10000,
      'committing should clear the awaiting notice',
    )

    await bystander.get(topicUrl)
    await bystander.wait(
      async () => (await mainText(bystander)).includes('This one waits for a moderator.'),
      10000,
      'a committed topic should be readable by anyone',
    )

    await bystander.get(`${baseUrl}/s/general/g/${groupSlug}`)
    await bystander.wait(
      async () => (await mainText(bystander)).includes(topicTitle),
      10000,
      'a committed topic should be listed on its group page',
    )

    await mod.get(topicUrl)
    await clickWhenReady(mod, By.xpath("//summary[text()='Premoderation']"), 'the premoderation panel should be present')
    await clickWhenReady(mod, By.xpath("//button[text()='Return to queue']"), 'a moderator should be able to send it back')
    await mod.wait(
      async () => (await mainText(mod)).includes('Awaiting moderation.'),
      10000,
      'returning it to the queue should show the awaiting notice again',
    )

    await bystander.get(topicUrl)
    await bystander.wait(until.elementLocated(By.css('main')), 10000)
    assert(
      !(await mainText(bystander)).includes('This one waits for a moderator.'),
      'a topic returned to the queue must be hidden again',
    )

    console.log('e2e: groups and premoderation flow passed')
  } finally {
    await mod.quit()
    await author.quit()
    await bystander.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
