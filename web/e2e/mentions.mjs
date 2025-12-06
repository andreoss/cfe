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

async function navText(driver, expected) {
  await driver.get(`${baseUrl}/`)
  await driver.wait(until.elementLocated(By.css('nav')), 10000)
  let settled = ''
  await driver.wait(
    async () => {
      const text = await driver.findElement(By.css('nav')).getText()
      if (!text.includes('Notifications') && !text.includes('Sign in')) return false
      if (expected !== undefined && !text.includes(expected)) return false
      settled = text
      return true
    },
    10000,
    `the navigation should come to show ${expected ?? 'a settled state'}`,
  )
  return settled
}

async function run() {
  const author = await buildDriver()
  const named = await buildDriver()
  const bystander = await buildDriver()
  const suffix = Date.now().toString(36)
  const authorName = `e2e_mena_${suffix}`
  const namedName = `e2e_menb_${suffix}`
  const otherName = `e2e_menc_${suffix}`
  const title = `A subject for mentions ${suffix}`
  try {
    await register(author, authorName)
    await register(named, namedName)
    await register(bystander, otherName)

    await author.get(`${baseUrl}/s/general`)
    await author.wait(
      until.elementLocated(By.xpath("//button[text()='New topic']")),
      10000,
      'the new topic control should appear once posting is known to be allowed',
    )
    await author.findElement(By.xpath("//button[text()='New topic']")).click()
    await author.wait(until.elementLocated(By.name('title')), 10000)
    await author.findElement(By.name('title')).sendKeys(title)
    await author.findElement(By.name('body')).sendKeys('A body for the mentions spec.')
    await clickWhenReady(author, By.xpath("//button[text()='Post']"), 'the post control')
    await publishTopic(title)
    await author.get(`${baseUrl}/s/general`)
    await author.wait(until.elementLocated(By.linkText(title)), 10000)
    await author.findElement(By.linkText(title)).click()
    await author.wait(until.elementLocated(By.name('comment-body')), 10000)
    const topicUrl = await author.getCurrentUrl()

    await author
      .findElement(By.name('comment-body'))
      .sendKeys(`I agree with @${namedName} on this`)
    await clickWhenReady(author, By.xpath("//button[text()='Post comment']"), 'the comment control')
    await author.wait(
      until.elementLocated(By.xpath(`//a[normalize-space(text())='@${namedName}']`)),
      10000,
      'a name in a remark should read as a link',
    )
    const mention = await author.findElement(
      By.xpath(`//a[normalize-space(text())='@${namedName}']`),
    )
    assert(
      (await mention.getAttribute('href')).endsWith(`/u/${namedName}`),
      'a mention should lead to whoever was named',
    )

    const namedNav = await navText(named, 'Notifications (1)')
    assert(
      namedNav.includes('Notifications (1)'),
      `somebody named should be told, nav was: ${namedNav}`,
    )

    const otherNav = await navText(bystander)
    assert(
      !otherNav.includes('Notifications ('),
      `somebody not named should not be told, nav was: ${otherNav}`,
    )

    await named.get(`${baseUrl}/notifications`)
    await named.wait(
      async () => (await mainText(named)).includes(authorName),
      10000,
      'the notice should name who wrote it',
    )

    await author.get(topicUrl)
    await author.findElement(By.name('comment-body')).sendKeys('write to someone@example.com')
    await clickWhenReady(author, By.xpath("//button[text()='Post comment']"), 'the comment control')
    await author.wait(
      async () => (await mainText(author)).includes('someone@example.com'),
      10000,
      'the address should be posted',
    )
    const addressMentions = await author.findElements(
      By.xpath("//a[contains(@class, 'mention') and contains(text(), '@example.com')]"),
    )
    assert(addressMentions.length === 0, 'an address must not be read as a mention')

    await clickWhenReady(
      author,
      By.xpath(`//a[normalize-space(text())='@${namedName}']`),
      'the mention link',
    )
    await author.wait(until.urlContains(`/u/${namedName}`), 10000)

    console.log('e2e: mentions flow passed')
  } finally {
    await author.quit()
    await named.quit()
    await bystander.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
